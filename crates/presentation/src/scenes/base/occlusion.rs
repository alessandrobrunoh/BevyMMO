//! Camera occlusion handling for the isometric game camera.
//!
//! Props are authored as two glTF nodes:
//! - `<Name>_Base` (trunk, floor) — always rendered.
//! - `<Name>_Top`  (canopy, roof) — hidden when it blocks the line of sight
//!   between the game camera and the locally controlled player.
//!
//! The `_Top` suffix is the only authoring contract: when Bevy instantiates a
//! scene, every glTF node becomes an entity that retains its `Name`, so the
//! [`tag_occludable_tops`] system can promote them to [`OccludableTop`]
//! automatically with no per-asset setup. See `plans/occlusion_fading_plan.md`
//! for the full rationale (visibility toggle vs. alpha blend, radius
//! auto-detection via Bevy's `Aabb`).

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use lightyear::prelude::Controlled;

use bevymmo_shared::network::protocol::Position;

use super::systems::GameCamera;

/// Suffix on a glTF node `Name` that marks a canopy/roof as occludable.
///
/// Mirrors the existing `WALKABLE_NODE_PREFIX` convention from
/// `bevymmo_shared::world::loader`: authoring rules live in the asset, the
/// client just reads them.
pub const OCCLUDABLE_TOP_SUFFIX: &str = "_Top";

/// Conservative radius (in world units) used while an occluder's mesh asset is
/// still streaming in and Bevy has not yet attached its `Aabb`.
///
/// Picked small enough to avoid over-hiding during the loading window, large
/// enough that a freshly spawned tree next to the player does not flash its
/// canopy before the first AABB lands.
const DEFAULT_OCCLUDER_RADIUS: f32 = 2.0;

/// Query type for freshly added named nodes that are not yet tagged.
///
/// Factored out to keep clippy's `type_complexity` lint quiet.
type NewNamedQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Name), (Added<Name>, Without<OccludableTop>)>;

/// Marks a scene node as the occludable "Top" of a prop.
///
/// Inserted automatically by [`tag_occludable_tops`] when the node `Name`
/// ends with [`OCCLUDABLE_TOP_SUFFIX`]. Carries no per-instance data: the
/// occlusion radius is derived at runtime from Bevy's `Aabb` (see
/// [`occluder_world_radius`]) so it tracks whatever scale the `#[props]`
/// macro and the manifest apply.
///
/// # Example
/// ```ignore
/// // Authored in Blender as a node named "Yggdrasil_Top".
/// // At runtime, the scene instance carries the `Name` and the tagging
/// // system marks it:
/// commands.entity(entity).insert(OccludableTop);
/// ```
#[derive(Component, Reflect, Clone, Copy, Default)]
#[reflect(Component)]
pub struct OccludableTop;

/// Promotes freshly instantiated glTF nodes ending with `_Top` to
/// [`OccludableTop`].
///
/// Runs on `Added<Name>` so it fires once per scene node the first frame it
/// appears (initial spawn or re-entry into `InGame`). Idempotent: the query
/// filters out entities already tagged with `OccludableTop`, so a second
/// insert never happens.
pub fn tag_occludable_tops(mut commands: Commands, new_named: NewNamedQuery) {
    for (entity, name) in &new_named {
        if name.as_str().ends_with(OCCLUDABLE_TOP_SUFFIX) {
            commands.entity(entity).insert(OccludableTop);
        }
    }
}

/// Hides occludable tops that block the line of sight between the game camera
/// and the locally controlled player.
///
/// Algorithm (per occluder, all in world space):
/// 1. **Resolve radius** via [`occluder_world_radius`] (bounding-sphere from
///    Bevy's `Aabb`, scaled by the entity's global transform).
/// 2. **Distance cull**: if the occluder is farther from the camera than the
///    player plus its own radius, it cannot overlap the camera→player segment.
/// 3. **Ray projection**: project the occluder center onto the normalized
///    camera→player ray.
/// 4. **Segment test**: hide when the projection lands between camera and
///    player (`0 < projection < distance`) and the perpendicular distance
///    from the ray is below `radius`.
/// 5. **Player-under-canopy**: also hide when the player itself is inside the
///    occluder's radius, so the canopy does not cover the character when
///    standing under the tree.
///
/// Thread-safety: runs on the main schedule, single-threaded per Bevy's
/// `Update`. No I/O, no allocations.
pub fn update_camera_occlusion(
    player_query: Query<&Position, With<Controlled>>,
    camera_query: Query<&Transform, With<GameCamera>>,
    mut occludables: Query<(
        &GlobalTransform,
        Option<&Aabb>,
        &OccludableTop,
        &mut Visibility,
    )>,
) {
    let Ok(player_position) = player_query.single() else {
        return;
    };
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation;
    let player_pos = player_position.0;
    let camera_to_player = player_pos - camera_pos;
    let camera_to_player_dist = camera_to_player.length();
    if camera_to_player_dist < f32::EPSILON {
        return;
    }
    let camera_to_player_dir = camera_to_player / camera_to_player_dist;

    for (transform, aabb, _occludable, mut visibility) in occludables.iter_mut() {
        let radius = occluder_world_radius(transform, aabb);
        let occluder_pos = transform.translation();

        // Step 1: distance cull — occluder is past the player from the
        // camera's POV, no chance to overlap the camera→player segment.
        let camera_to_occluder_dist = camera_pos.distance(occluder_pos);
        if camera_to_occluder_dist > camera_to_player_dist + radius {
            *visibility = Visibility::Inherited;
            continue;
        }

        // Step 2 & 3: project the occluder center onto the camera→player ray.
        let to_occluder = occluder_pos - camera_pos;
        let projection = to_occluder.dot(camera_to_player_dir);
        let between_camera_and_player = projection > 0.0 && projection < camera_to_player_dist;

        let is_blocking_segment = if between_camera_and_player {
            let closest_point_on_ray = camera_pos + camera_to_player_dir * projection;
            closest_point_on_ray.distance(occluder_pos) < radius
        } else {
            false
        };

        // Step 4: player standing under the canopy should also reveal the
        // character.
        let player_under_canopy = occluder_pos.distance(player_pos) < radius;

        *visibility = if is_blocking_segment || player_under_canopy {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

/// Derives the world-space bounding-sphere radius of an occluder.
///
/// Uses Bevy's automatically-computed `Aabb` (attached by `bevy_render` to
/// every `Mesh3d` entity) and scales it by the entity's global transform so
/// the result reflects the actual in-game size — including any per-placement
/// scale from the manifest and the `scale = (...)` attribute of the `#[props]`
/// macro.
///
/// Returns [`DEFAULT_OCCLUDER_RADIUS`] when `aabb` is `None` (asset still
/// streaming in). This keeps occlusion stable during the brief loading window
/// rather than letting canopies pop fully visible for a frame or two.
fn occluder_world_radius(transform: &GlobalTransform, aabb: Option<&Aabb>) -> f32 {
    let Some(aabb) = aabb else {
        return DEFAULT_OCCLUDER_RADIUS;
    };
    // `Aabb::half_extents` is in the entity's local space. Multiplying by the
    // global transform's scale yields world-space half extents; their length
    // is the bounding-sphere radius (tight for spheres, conservative for
    // elongated canopies — which is what we want for occlusion: prefer to
    // hide a bit too eagerly than to leak the player).
    let world_scale = transform.compute_transform().scale;
    let world_half_extents = aabb.half_extents * Vec3A::from(world_scale);
    world_half_extents.length()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::math::Vec3A;

    /// Spawns an occluder at `translation` with a unit-scale global transform
    /// and a known local-space `half_extent` (so the bounding-sphere radius is
    /// `half_extent * sqrt(3)`).
    fn occluder_entity(world: &mut World, translation: Vec3, half_extent: f32) -> Entity {
        world
            .spawn((
                Transform::from_translation(translation),
                GlobalTransform::from_translation(translation),
                Visibility::Inherited,
                OccludableTop,
                Aabb {
                    center: Vec3A::ZERO,
                    half_extents: Vec3A::splat(half_extent),
                },
            ))
            .id()
    }

    fn visibility_of(world: &mut World, entity: Entity) -> Visibility {
        *world
            .entity(entity)
            .get::<Visibility>()
            .expect("occluder has Visibility")
    }

    /// Spawns a `GameCamera` at `camera` and a `Controlled` player at `player`,
    /// then runs [`update_camera_occlusion`] once.
    fn run_occlusion(world: &mut World, camera: Vec3, player: Vec3) {
        world.spawn((
            GameCamera,
            Transform::from_translation(camera),
            Camera3d::default(),
        ));
        world.spawn((Controlled, Position(player)));
        world
            .run_system_once(update_camera_occlusion)
            .expect("system runs");
    }

    #[test]
    fn occluder_between_camera_and_player_is_hidden() {
        // Camera looks down the +Z axis toward the player.
        let camera = Vec3::new(0.0, 0.0, -10.0);
        let player = Vec3::new(0.0, 0.0, 10.0);
        let mut world = World::new();

        // Occluder halfway, dead-center on the camera→player segment. Half
        // extent 5.0 → bounding radius ~8.66, comfortably larger than the
        // perpendicular distance (0).
        let entity = occluder_entity(&mut world, Vec3::new(0.0, 0.0, 0.0), 5.0);

        run_occlusion(&mut world, camera, player);
        assert_eq!(
            visibility_of(&mut world, entity),
            Visibility::Hidden,
            "occluder on the segment must be hidden"
        );
    }

    #[test]
    fn occluder_off_axis_stays_visible() {
        let camera = Vec3::new(0.0, 0.0, -10.0);
        let player = Vec3::new(0.0, 0.0, 10.0);
        let mut world = World::new();

        // Half extent 1.0 → bounding radius ~1.73. Occluder sits 50 units to
        // the side, far outside the segment's tolerance.
        let entity = occluder_entity(&mut world, Vec3::new(50.0, 0.0, 0.0), 1.0);

        run_occlusion(&mut world, camera, player);
        assert_eq!(
            visibility_of(&mut world, entity),
            Visibility::Inherited,
            "off-axis occluder must stay visible"
        );
    }

    #[test]
    fn occluder_behind_player_stays_visible() {
        let camera = Vec3::new(0.0, 0.0, -10.0);
        let player = Vec3::new(0.0, 0.0, 10.0);
        let mut world = World::new();

        // Occluder lies past the player along the same axis.
        let entity = occluder_entity(&mut world, Vec3::new(0.0, 0.0, 20.0), 1.0);

        run_occlusion(&mut world, camera, player);
        assert_eq!(
            visibility_of(&mut world, entity),
            Visibility::Inherited,
            "occluder behind the player must stay visible"
        );
    }

    #[test]
    fn player_inside_canopy_is_hidden() {
        // Isometric-style camera, player at the origin.
        let camera = Vec3::new(0.0, 25.0, 25.0);
        let player = Vec3::ZERO;
        let mut world = World::new();

        // Canopy half extent 5.0 → bounding radius ~8.66, centered 4 units
        // above the player (typical tree placement): player is inside.
        let entity = occluder_entity(&mut world, Vec3::new(0.0, 4.0, 0.0), 5.0);

        run_occlusion(&mut world, camera, player);
        assert_eq!(
            visibility_of(&mut world, entity),
            Visibility::Hidden,
            "canopy over the player must hide so the character stays visible"
        );
    }

    #[test]
    fn occlusion_noops_without_controlled_player() {
        let mut world = World::new();
        world.spawn((GameCamera, Transform::from_translation(Vec3::ZERO)));

        let entity = occluder_entity(&mut world, Vec3::ZERO, 1.0);

        world
            .run_system_once(update_camera_occlusion)
            .expect("system runs");
        assert_eq!(
            visibility_of(&mut world, entity),
            Visibility::Inherited,
            "no controlled player -> occlusion must not flip visibility"
        );
    }

    #[test]
    fn tag_occludable_tops_only_tags_top_suffixed_names() {
        let mut world = World::new();
        let top_entity = world.spawn(Name::new("Yggdrasil_Top")).id();
        let base_entity = world.spawn(Name::new("Yggdrasil_Base")).id();
        let other_entity = world.spawn(Name::new("Rock")).id();

        world
            .run_system_once(tag_occludable_tops)
            .expect("system runs");

        assert!(
            world.entity(top_entity).contains::<OccludableTop>(),
            "_Top-suffixed node must be tagged"
        );
        assert!(
            !world.entity(base_entity).contains::<OccludableTop>(),
            "_Base node must not be tagged"
        );
        assert!(
            !world.entity(other_entity).contains::<OccludableTop>(),
            "unrelated node must not be tagged"
        );
    }

    #[test]
    fn radius_scales_with_global_transform() {
        // Half extent 1.0 in local space, but the entity is scaled ×3 in the
        // world → world-space bounding radius must be 3 * sqrt(3) ~= 5.196.
        let transform = Transform::from_translation(Vec3::ZERO).with_scale(Vec3::splat(3.0));
        let global = GlobalTransform::from(transform);
        let aabb = Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::splat(1.0),
        };

        let radius = occluder_world_radius(&global, Some(&aabb));
        let expected = 3.0_f32 * 3.0_f32.sqrt();
        assert!(
            (radius - expected).abs() < 1e-4,
            "expected radius {expected}, got {radius}"
        );
    }

    #[test]
    fn radius_falls_back_to_default_without_aabb() {
        let transform = GlobalTransform::IDENTITY;

        let radius = occluder_world_radius(&transform, None);
        assert_eq!(
            radius, DEFAULT_OCCLUDER_RADIUS,
            "missing Aabb must fall back to DEFAULT_OCCLUDER_RADIUS"
        );
    }
}
