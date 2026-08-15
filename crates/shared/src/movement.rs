//! Shared pure helpers for point-and-click movement.
//!
//! Contains the canonical movement-speed computation, the cast-blocking
//! policy, and the shared move-towards-target stepping used by both the
//! authoritative server system and the client-side prediction system.

use bevy::prelude::{Mut, Resource, Vec3};

use crate::entity::components::EntityState;
use crate::network::protocol::{Inputs, LookDirection, Position};
use crate::spells::{CastKind, CastProgress};
use crate::stats::events::ModifierOp;
use crate::stats::events::StatField;
use crate::stats::modifiers::ActiveStatModifiers;
use crate::stats::modifiers::StatModifierInstance;
use crate::world::{CollisionGrid, SurfaceQuery};

/// Distance (in world units) under which a move command is considered satisfied.
pub const ARRIVAL_DISTANCE: f32 = 0.05;

/// Local pending click target shared between click selection, input buffering,
/// and the predicted/authoritative move systems.
///
/// Pure data: lives in `shared` so both `bevymmo_server` and `bevymmo_client`
/// can use it without creating a cross-crate dependency.
#[derive(Resource, Default)]
pub struct MoveTarget(pub Option<Vec3>);

/// Client-side surface query data for height-aware click-to-move.
///
/// Populated by the presentation layer from loaded map data, consumed by the
/// client click-to-move system to resolve mouse clicks onto proper terrain heights.
/// Lives in `shared` so both client and presentation can access it without
/// cross-crate dependencies.
#[derive(Resource, Default)]
pub struct ClientSurfaceQuery(pub Option<SurfaceQuery>);

/// Calculates movement speed after active stat modifiers.
///
/// This is shared by gameplay and the stats UI so the value displayed to the
/// player matches the speed used by gameplay.
pub fn effective_movement_speed(base_speed: f32, modifiers: Option<&ActiveStatModifiers>) -> f32 {
    let Some(active) = modifiers else {
        return base_speed;
    };
    effective_value(StatField::Speed, base_speed, &active.modifiers)
}

fn effective_value(field: StatField, base: f32, modifiers: &[StatModifierInstance]) -> f32 {
    let mut result = base;
    let mut override_value: Option<f32> = None;

    for modifier in modifiers {
        for effect in &modifier.effects {
            if let crate::stats::modifiers::ModifierEffectInstance::Stat {
                field: effect_field,
                operation,
                value,
            } = effect
            {
                if *effect_field != field {
                    continue;
                }
                match operation {
                    ModifierOp::Add => result += value,
                    ModifierOp::Multiply => result *= value,
                    ModifierOp::Override => override_value = Some(*value),
                }
            }
        }
    }

    override_value.unwrap_or(result)
}

/// Returns true when a cast state must freeze point-and-click movement.
pub fn should_block_movement_for_cast(cast: Option<&CastProgress>) -> bool {
    let Some(cast) = cast else {
        return false;
    };
    match cast.kind {
        CastKind::CastTime => true,
        CastKind::Channeling => {
            cast.channel_movement == crate::spells::ChannelMovementPolicy::InterruptOnMove
        }
        CastKind::Instant => false,
    }
}

/// Steps a single entity towards its current move target.
///
/// Shared by the authoritative server system (`bevymmo_server::player_movement`)
/// and the client prediction system (`bevymmo_presentation::player_movement`)
/// so both sides advance movement with identical math.
///
/// Returns early if the entity is dead; clears state to `Idle` when the input
/// is not a `MoveTo` or the entity has reached the target.
pub fn move_towards_target(
    mut position: Mut<Position>,
    mut look_direction: Mut<LookDirection>,
    input: &Inputs,
    speed: f32,
    mut state: Mut<EntityState>,
) {
    if state.is_dead() {
        return;
    }

    let Inputs::MoveTo(target) = input else {
        *state = EntityState::Idle;
        return;
    };

    let offset = *target - position.0;
    let distance = offset.length();
    if distance > 0.001 {
        look_direction.0 = (offset / distance).normalize_or_zero();
    }
    if distance <= ARRIVAL_DISTANCE {
        position.0 = *target;
        *state = EntityState::Idle;
        return;
    }

    position.0 += offset / distance * speed.min(distance);
    *state = EntityState::Moving;
}

// ==================== HEIGHT-AWARE MOVEMENT HELPERS ====================

/// Resolves a 2D position (x, z) to a 3D position (x, y, z) using surface queries.
///
/// Returns `None` if the position is not over any walkable surface.
/// Returns `Some(Vec3)` with the resolved height if a surface is found.
///
/// This is the main helper for height-aware movement, allowing 2D movement
/// commands (like click-to-move) to be resolved to proper 3D positions on terrain.
pub fn resolve_ground_position(x: f32, z: f32, surface_query: &SurfaceQuery) -> Option<Vec3> {
    surface_query
        .ground_at(x, z)
        .map(|contact| Vec3::new(x, contact.height, z))
}

/// Validates that movement between two points is valid.
///
/// Checks that both the start and end positions are on walkable surfaces,
/// and that the movement doesn't cross blocking obstacles.
///
/// Returns `true` if movement is valid, `false` otherwise.
pub fn is_valid_movement(
    from: Vec3,
    to: Vec3,
    surface_query: &SurfaceQuery,
    collision_grid: &crate::world::CollisionGrid,
) -> bool {
    // Check if both start and end positions are on walkable surfaces
    let from_contact = surface_query.ground_at(from.x, from.z);
    let to_contact = surface_query.ground_at(to.x, to.z);

    if from_contact.is_none() || to_contact.is_none() {
        return false;
    }

    // Check if the movement crosses any blocking obstacles
    // For now, just check the end point against obstacles
    // In a full implementation, we would check the path between points
    if collision_grid.is_blocked([to.x, to.y, to.z], 0.35) {
        return false;
    }

    true
}

/// Steps a single entity towards its 2D move target using surface queries for height.
///
/// This is similar to `move_towards_target()` but uses surface queries to
/// resolve the proper height for the target position, making it suitable for
/// height-aware movement on terrain with varying elevation.
///
/// Returns `Some(new_position)` if movement succeeded, `None` if the target
/// is not on a walkable surface.
pub fn step_towards_2d_target(
    current_position: Vec3,
    target_x: f32,
    target_z: f32,
    speed: f32,
    surface_query: &SurfaceQuery,
) -> Option<Vec3> {
    // Resolve the target height using surface queries
    let target_height = surface_query.ground_at(target_x, target_z)?;
    let target = Vec3::new(target_x, target_height.height, target_z);

    // Calculate movement direction and distance
    let offset = target - current_position;
    let distance = offset.length();

    if distance <= ARRIVAL_DISTANCE {
        return Some(target); // Arrived at target
    }

    // Step towards target
    let step_distance = speed.min(distance);
    let new_position = current_position + offset.normalize() * step_distance;

    // Resolve height at the new position
    let new_height = surface_query.ground_at(new_position.x, new_position.z);
    match new_height {
        Some(contact) => Some(Vec3::new(new_position.x, contact.height, new_position.z)),
        None => Some(new_position), // Fallback to unmodified position if no surface
    }
}

// ==================== GROUND SNAP ====================

/// Permissive vertical budget used by [`snap_to_ground`] when the caller does
/// not have a manifest-derived `max_step_height` handy.
///
/// It is intentionally generous: snap is a *recovery* path for spawn,
/// respawn and teleport, so we want to absorb large drifts (a player loaded
/// from the DB at y=0 onto a raised hill). Capping it prevents the documented
/// teleport-to-top-surface bug on overlapping surfaces while still allowing
/// the common "persisted Y is off by a few metres" case to recover.
pub const SNAP_STEP_BUDGET: f32 = 5.0;

/// Snaps an entity's Y to the highest reachable ground surface at its current
/// XZ position.
///
/// Returns `true` if the Y was changed. This is the recovery path for spawn,
/// respawn, teleport, knockback, or any other code path that writes `Position`
/// without resolving the terrain height.
///
/// Unlike the previous `ground_at` based implementation, this uses
/// [`SurfaceQuery::ground_at_reachable`] so that overlapping surfaces above
/// `current_y + max_step_height` cannot teleport the entity onto the wrong
/// platform (e.g. a player walking under a bridge ending up on the bridge).
/// Surfaces *below* the entity are always reachable, so the recovery still
/// snaps a stranded entity (e.g. spawned below the surface) back down onto it.
///
/// On a flat surface (no height data), this is a no-op.
///
/// # Arguments
///
/// * `max_step_height` - Vertical reach budget for the snap. Pass
///   [`SNAP_STEP_BUDGET`] for generic recovery, or the manifest's
///   `world_metrics.max_step_height` when you want spawn to honour the same
///   step rule as live movement.
pub fn snap_to_ground(
    position: &mut Vec3,
    surface_query: &SurfaceQuery,
    max_step_height: f32,
) -> bool {
    let contact = surface_query
        .ground_at_reachable(position.x, position.z, position.y, max_step_height)
        // Stranded *below* the terrain: `ground_at_reachable` rejects every
        // surface higher than `current_y + max_step_height`, so an entity that
        // ends up under the ground has no reachable surface at all and stays
        // there forever — it cannot move either, because every candidate step
        // fails the same test. This happens whenever a position is written
        // without consulting the terrain: a new player at the default
        // `Vec3::ZERO` on a map whose origin is a hillside, a player whose
        // persisted coordinates predate a terrain edit, or a teleport.
        //
        // Recovery is the whole point of this function, so fall back to the
        // highest surface at the entity's XZ and lift it out. Live stepping
        // still goes through `ground_at_reachable` in `try_step`, so this
        // cannot be used to climb a cliff.
        .or_else(|| surface_query.ground_at(position.x, position.z));
    let Some(contact) = contact else {
        return false;
    };
    if (contact.height - position.y).abs() > 0.001 {
        position.y = contact.height;
        return true;
    }
    false
}

// ==================== TERRAIN-AWARE STEPPING ====================

/// Outcome of a single terrain-aware movement step.
///
/// Shared between the authoritative server system and the client prediction
/// system so both sides interpret the step identically.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerrainStep {
    /// Entity reached its target; carry the resolved on-ground position.
    Arrived(Vec3),
    /// Entity moved one step toward the target.
    Moved(Vec3),
    /// Step was rejected: blocked by an obstacle, too steep, or off-surface.
    Blocked,
    /// Target is not on any walkable surface.
    NoSurface,
}

/// Default collision radius used by the terrain stepper when probing blockers.
const STEP_COLLISION_RADIUS: f32 = 0.45;

/// Steps an entity toward a 2D target (x, z) on a height-aware world.
///
/// This is the canonical terrain-aware stepper used by both the authoritative
/// server system and the client prediction system. It differs from the older
/// `move_towards_target` / `step_towards_2d_target` helpers in three important
/// ways that together fix the broken climbing behaviour:
///
/// 1. **Reachability-filtered surface selection** — at the candidate next
///    position, only surfaces whose height is within `max_step_height` of the
///    entity's *current* Y are considered (see [`SurfaceQuery::ground_at_reachable`]),
///    and among those the highest one wins. This prevents the entity from
///    snapping onto a mountain corridor merely because its bounding rectangle
///    overlaps the player's position, while still letting it climb ramps and
///    switchbacks one tick at a time.
/// 2. **`max_step_height` enforcement** — implicit in (1): a candidate step
///    whose vertical delta exceeds `max_step_height` is rejected, so a single
///    tick can no longer teleport the entity up (or down) a cliff.
/// 3. **Sliding along obstacles** — if the straight-line step is blocked by
///    an obstacle, the stepper retries with the X-only and Z-only components
///    of the desired direction so the entity slides along walls and cliff
///    blockers instead of freezing in place. This is what lets a player walk
///    up a switchback corridor whose direct line is walled off.
///
/// `speed` is the per-tick horizontal travel distance.
///
/// [`SurfaceQuery::ground_at_reachable`]: crate::world::SurfaceQuery::ground_at_reachable
pub fn step_on_terrain(
    current: Vec3,
    target_x: f32,
    target_z: f32,
    speed: f32,
    surface_query: &SurfaceQuery,
    collision_grid: &CollisionGrid,
    max_step_height: f32,
) -> TerrainStep {
    // Resolve the target surface with highest-wins: we want the actual goal
    // height (e.g. the mountain summit), not a nearby lower surface.
    let target_contact = match surface_query.ground_at(target_x, target_z) {
        Some(contact) => contact,
        None => return TerrainStep::NoSurface,
    };

    // Horizontal-only direction toward the target (Y is resolved by terrain).
    let dx = target_x - current.x;
    let dz = target_z - current.z;
    let horizontal_distance = (dx * dx + dz * dz).sqrt();

    if horizontal_distance <= ARRIVAL_DISTANCE {
        return TerrainStep::Arrived(Vec3::new(target_x, target_contact.height, target_z));
    }

    let step = speed.min(horizontal_distance);
    let nx = dx / horizontal_distance;
    let nz = dz / horizontal_distance;

    // Direct step.
    if let Some(pos) = try_step(
        current,
        nx,
        nz,
        step,
        surface_query,
        collision_grid,
        max_step_height,
    ) {
        return TerrainStep::Moved(pos);
    }

    // Slide attempts: drop one axis at a time so the entity can glide along
    // a wall or cliff blocker instead of stopping dead.
    //
    // Try the *smaller* axis first: if the player is moving mostly along X
    // and hits a wall perpendicular to X, we want to keep the X component and
    // drop Z, and vice-versa. Trying both orderings would also work, but
    // preferring the dominant axis keeps movement feeling natural on slopes.
    let (first, second) = if nx.abs() >= nz.abs() {
        ((nx, 0.0), (0.0, nz))
    } else {
        ((0.0, nz), (nx, 0.0))
    };

    for (sx, sz) in [first, second] {
        if sx.abs() < 1e-6 && sz.abs() < 1e-6 {
            continue;
        }
        if let Some(pos) = try_step(
            current,
            sx,
            sz,
            step,
            surface_query,
            collision_grid,
            max_step_height,
        ) {
            return TerrainStep::Moved(pos);
        }
    }

    TerrainStep::Blocked
}

/// Tries a single step in a given (not necessarily normalized) horizontal
/// direction. Returns the resolved on-ground position if the step is valid,
/// or `None` if it is blocked, off-surface, or too steep.
fn try_step(
    current: Vec3,
    dir_x: f32,
    dir_z: f32,
    step: f32,
    surface_query: &SurfaceQuery,
    collision_grid: &CollisionGrid,
    max_step_height: f32,
) -> Option<Vec3> {
    let len = (dir_x * dir_x + dir_z * dir_z).sqrt();
    if len < 1e-6 {
        return None;
    }

    let next_x = current.x + dir_x / len * step;
    let next_z = current.z + dir_z / len * step;

    // Pick the highest reachable surface at the candidate position. This is
    // the core of the climbing fix: the entity can only step onto a surface
    // within `max_step_height` of its current Y, so it follows ramps and
    // switchbacks one tick at a time and never teleports up a cliff or onto
    // an unrelated overlapping surface.
    let next_contact =
        surface_query.ground_at_reachable(next_x, next_z, current.y, max_step_height)?;

    let candidate = Vec3::new(next_x, next_contact.height, next_z);

    if collision_grid.is_blocked(
        [candidate.x, candidate.y, candidate.z],
        STEP_COLLISION_RADIUS,
    ) {
        return None;
    }

    Some(candidate)
}

// ==================== RAY-TO-SURFACE RESOLUTION ====================

/// Resolves a camera ray to a ground position on walkable surfaces.
///
/// Samples points along the ray from the camera and returns the first point
/// whose X/Z coordinates resolve to a valid ground contact via `SurfaceQuery`.
/// This enables height-aware click-to-move without relying on visual mesh raycasts.
///
/// # Arguments
/// * `ray_origin` - The camera position in world space
/// * `ray_direction` - Normalized direction vector from camera through cursor
/// * `surface_query` - Surface query data for terrain height resolution
/// * `max_distance` - Maximum distance to sample along the ray (default 100.0)
/// * `step_size` - Distance between sample points along the ray (default 1.0)
///
/// # Returns
/// * `Some(Vec3)` - First valid ground position on the ray
/// * `None` - No valid ground position found within max_distance
///
/// # Example
/// ```ignore
/// let target = resolve_ray_to_ground(
///     camera_pos,
///     ray_dir,
///     &surface_query,
///     100.0,
///     1.0
/// );
/// ```
pub fn resolve_ray_to_ground(
    ray_origin: Vec3,
    ray_direction: Vec3,
    surface_query: &SurfaceQuery,
    max_distance: f32,
    step_size: f32,
) -> Option<Vec3> {
    if surface_query.is_empty() {
        // Fallback to Y=0 plane when no surface data is available
        let plane_normal = Vec3::Y;
        let plane_d = 0.0; // Y = 0 plane

        // Ray-plane intersection: t = -(normal · origin + d) / (normal · direction)
        let denominator = plane_normal.dot(ray_direction);
        if denominator.abs() < 1e-6 {
            return None; // Ray is parallel to plane
        }

        let t = -(plane_normal.dot(ray_origin) + plane_d) / denominator;
        if t < 0.0 || t > max_distance {
            return None; // Intersection is behind camera or too far
        }

        let intersection = ray_origin + ray_direction * t;
        return Some(Vec3::new(intersection.x, 0.0, intersection.z));
    }

    let normalized_direction = ray_direction.normalize_or_zero();
    if normalized_direction == Vec3::ZERO {
        return None;
    }

    // Find where the camera ray actually crosses the terrain. Merely returning
    // the first sample whose X/Z is inside the map selects a point near the
    // camera on hills, instead of the point under the cursor.
    let num_steps = (max_distance / step_size).ceil() as i32;
    let mut previous: Option<(f32, f32)> = None;

    for step in 0..=num_steps {
        let t = (step as f32 * step_size).min(max_distance);
        let sample_point = ray_origin + normalized_direction * t;
        let Some(ground_contact) = surface_query.ground_at(sample_point.x, sample_point.z) else {
            previous = None;
            continue;
        };

        let signed_distance = sample_point.y - ground_contact.height;
        if let Some((previous_t, previous_distance)) = previous {
            if previous_distance >= 0.0 && signed_distance <= 0.0 {
                // Refine the crossing so coarse ray steps do not move the click
                // target noticeably on steep terrain.
                let mut low = previous_t;
                let mut high = t;
                for _ in 0..8 {
                    let middle = (low + high) * 0.5;
                    let point = ray_origin + normalized_direction * middle;
                    let Some(contact) = surface_query.ground_at(point.x, point.z) else {
                        low = middle;
                        continue;
                    };
                    if point.y - contact.height > 0.0 {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }

                let point = ray_origin + normalized_direction * high;
                let contact = surface_query.ground_at(point.x, point.z)?;
                return Some(Vec3::new(point.x, contact.height, point.z));
            }
        }

        previous = Some((t, signed_distance));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{
        CollisionGrid, HeightfieldData, MapBounds, MapManifest, SurfaceBounds, SurfaceKind,
        SurfaceQuery, WalkableSurface, WorldMetrics,
    };

    fn create_test_surface_query() -> (SurfaceQuery, MapManifest) {
        let manifest = MapManifest {
            version: 2,
            map_id: "test_movement".to_string(),
            display_name: "Test Movement".to_string(),
            bounds: MapBounds {
                min_x: -20.0,
                max_x: 20.0,
                min_z: -20.0,
                max_z: 20.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![WalkableSurface {
                id: "surface_flat".to_string(),
                kind: SurfaceKind::Flat,
                object: None,
                bounds: Some(SurfaceBounds {
                    min_x: -10.0,
                    max_x: 10.0,
                    min_z: -10.0,
                    max_z: 10.0,
                }),
                height: Some(2.0),
                min_height: None,
                max_height: None,
                grid_size: None,
                size: Some(20.0),
                purpose: Some("Test surface for movement".to_string()),
                heightfield: None,
                walkable_mesh: None,
                layer: None,
                max_slope_deg: None,
            }],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        (SurfaceQuery::from_manifest(&manifest), manifest)
    }

    #[test]
    fn test_resolve_ground_position_on_surface() {
        let (query, _manifest) = create_test_surface_query();

        // Test resolving a position on the surface
        let pos = resolve_ground_position(0.0, 0.0, &query)
            .expect("in-bounds surface point should resolve to a 3D position");
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.z, 0.0);
        assert_eq!(pos.y, 2.0); // Should match the surface height
    }

    #[test]
    fn snap_to_ground_lifts_an_entity_stranded_below_the_terrain() {
        let (query, _manifest) = create_test_surface_query();

        // Surface sits at y = 2.0; the entity is 9 m under it, as happens to a
        // player spawned at the default `Vec3::ZERO` on a map whose origin is
        // a hillside, or one whose persisted Y predates a terrain edit.
        let mut position = Vec3::new(0.0, -7.0, 0.0);
        assert!(
            snap_to_ground(&mut position, &query, 0.45),
            "an entity below every surface must be recovered, not left stuck"
        );
        assert_eq!(position.y, 2.0);
    }

    #[test]
    fn snap_to_ground_still_refuses_positions_with_no_surface_at_all() {
        let (query, _manifest) = create_test_surface_query();

        let mut position = Vec3::new(100.0, -7.0, 100.0);
        assert!(!snap_to_ground(&mut position, &query, 0.45));
        assert_eq!(position.y, -7.0);
    }

    #[test]
    fn test_resolve_ground_position_off_surface() {
        let (query, _manifest) = create_test_surface_query();

        // Test resolving a position off the surface
        let result = resolve_ground_position(100.0, 100.0, &query);
        assert!(result.is_none());
    }

    #[test]
    fn test_step_towards_2d_target() {
        let (query, _manifest) = create_test_surface_query();

        let start = Vec3::new(-5.0, 2.0, -5.0);
        let target_x = 5.0;
        let target_z = 5.0;
        let speed = 1.0;

        // Take a step towards the target
        let new_pos = step_towards_2d_target(start, target_x, target_z, speed, &query)
            .expect("in-bounds target should produce a height-aware movement step");
        // Should have moved towards the target
        assert!(new_pos.x > start.x);
        assert!(new_pos.z > start.z);
        // Height should be resolved from the surface
        assert_eq!(new_pos.y, 2.0);
    }

    #[test]
    fn test_step_towards_2d_target_arrival() {
        let (query, _manifest) = create_test_surface_query();

        let start = Vec3::new(0.0, 2.0, 0.0);
        let target_x = 0.05; // Very close to start
        let target_z = 0.05;
        let speed = 1.0;

        // Should arrive at target immediately
        let new_pos = step_towards_2d_target(start, target_x, target_z, speed, &query)
            .expect("nearby in-bounds target should resolve immediately");
        // Should be at the target position (within arrival distance)
        assert!((new_pos.x - target_x).abs() < ARRIVAL_DISTANCE);
        assert!((new_pos.z - target_z).abs() < ARRIVAL_DISTANCE);
    }

    #[test]
    fn test_step_towards_2d_invalid_target() {
        let (query, _manifest) = create_test_surface_query();

        let start = Vec3::new(0.0, 2.0, 0.0);
        let target_x = 100.0; // Off surface
        let target_z = 100.0;
        let speed = 1.0;

        // Should fail because target is not on a walkable surface
        let result = step_towards_2d_target(start, target_x, target_z, speed, &query);
        assert!(result.is_none());
    }

    // ==================== TERRAIN STEP TESTS ====================

    /// Builds a tiny world with a flat ground at y=0.0 and a ramp that rises
    /// from y=0.0 at x=5.0 to y=5.0 at x=10.0 (1 unit of Y per 1 unit of X).
    /// The ramp overlaps the ground in XZ bounds, which is exactly the
    /// rolling-hills-vs-mountain scenario from the bug report.
    fn create_ramp_world() -> (SurfaceQuery, CollisionGrid) {
        let bounds = SurfaceBounds {
            min_x: -10.0,
            max_x: 10.0,
            min_z: -10.0,
            max_z: 10.0,
        };
        // 5x5 ramp heightfield: 0 at x=-10, 5 at x=10 (linear).
        let res = 5u32;
        let stride = (res + 1) as usize;
        let mut heights = vec![0.0f32; stride * stride];
        for xi in 0..=res as usize {
            let h = xi as f32; // 0..5
            for zi in 0..=res as usize {
                heights[xi * stride + zi] = h;
            }
        }
        let ramp_hf = HeightfieldData::new(res, bounds, heights);
        let manifest = MapManifest {
            version: 2,
            map_id: "ramp".to_string(),
            display_name: "Ramp".to_string(),
            bounds: MapBounds {
                min_x: -10.0,
                max_x: 10.0,
                min_z: -10.0,
                max_z: 10.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![WalkableSurface {
                id: "surface_ramp".to_string(),
                kind: SurfaceKind::Mesh,
                object: None,
                bounds: Some(bounds),
                height: None,
                min_height: None,
                max_height: None,
                grid_size: None,
                size: None,
                purpose: None,
                heightfield: Some(ramp_hf),
                walkable_mesh: None,
                layer: None,
                max_slope_deg: None,
            }],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };
        (
            SurfaceQuery::from_manifest(&manifest),
            CollisionGrid::build(&manifest),
        )
    }

    #[test]
    fn test_step_on_terrain_rejects_cliff_too_high() {
        // From y=0 a direct step toward a ramp point that is much higher than
        // max_step_height must NOT teleport the entity up: there is no
        // reachable surface at the candidate position within budget.
        let (query, grid) = create_ramp_world();

        // Player on the ground at y=0, near the steep top of the ramp where
        // the height delta per step exceeds the default 0.45 budget.
        let start = Vec3::new(8.0, 0.0, 0.0);
        // Step toward a point on the ramp at x=9 (height ≈ 4.5).
        let step = step_on_terrain(start, 9.0, 0.0, 1.0, &query, &grid, 0.45);
        assert_eq!(step, TerrainStep::Blocked);
    }

    #[test]
    fn test_step_on_terrain_climbs_ramp_gradually() {
        // Walking from the bottom of the ramp upward one tick at a time, the
        // entity should ascend without ever skipping more than max_step_height
        // in a single tick. This is the canonical "player walks up the
        // switchback" scenario.
        let (query, grid) = create_ramp_world();

        let mut pos = Vec3::new(-9.0, 0.0, 0.0); // ramp base, y=0
        let target = Vec3::new(9.0, 5.0, 0.0); // ramp top
        let max_step_height = 0.45;
        let speed = 0.5;

        let mut prev_y = pos.y;
        for _ in 0..200 {
            match step_on_terrain(
                pos,
                target.x,
                target.z,
                speed,
                &query,
                &grid,
                max_step_height,
            ) {
                TerrainStep::Arrived(p) => {
                    pos = p;
                    break;
                }
                TerrainStep::Moved(p) => {
                    let dy = (p.y - prev_y).abs();
                    assert!(
                        dy <= max_step_height + 1e-5,
                        "single-tick vertical delta {} exceeded max_step_height {}",
                        dy,
                        max_step_height
                    );
                    prev_y = p.y;
                    pos = p;
                }
                TerrainStep::Blocked | TerrainStep::NoSurface => break,
            }
        }

        // The entity should have climbed significantly, proving the ramp was
        // followed instead of the entity being stuck on the ground.
        assert!(
            pos.y > 2.0,
            "entity should have climbed the ramp, ended at y={}",
            pos.y
        );
    }

    // ==================== RAY-TO-SURFACE TESTS ====================

    #[test]
    fn test_resolve_ray_to_ground_flat_surface() {
        let (query, _manifest) = create_test_surface_query();

        // Camera above the surface, looking down at the center
        let camera_pos = Vec3::new(0.0, 10.0, 5.0);
        let ray_dir = (Vec3::new(0.0, 2.0, 0.0) - camera_pos).normalize();

        let result = resolve_ray_to_ground(camera_pos, ray_dir, &query, 100.0, 0.5);

        assert!(result.is_some(), "Ray should hit the flat surface");
        let ground_pos = result.unwrap();
        assert_eq!(ground_pos.y, 2.0, "Height should match surface height");
        // X should be near 0 (camera is centered on X)
        assert!((ground_pos.x - 0.0).abs() < 1.0, "X should be near 0");
        // Z should be closer to 0 than to camera position (5.0), indicating we hit the surface
        assert!(ground_pos.z < 4.0, "Z should be less than camera Z (5.0)");
        // Z should be reasonably close to 0 (the target we were aiming at)
        assert!(ground_pos.z > -1.0, "Z should be greater than -1.0");
    }

    #[test]
    fn test_resolve_ray_to_ground_fallback_to_y0_plane() {
        // Create an empty surface query (no surface data)
        let empty_query = SurfaceQuery::from_manifest(&MapManifest {
            version: 2,
            map_id: "empty".to_string(),
            display_name: "Empty Map".to_string(),
            bounds: MapBounds {
                min_x: -10.0,
                max_x: 10.0,
                min_z: -10.0,
                max_z: 10.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![], // No surfaces
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        });

        // Camera looking down at the Y=0 plane
        let camera_pos = Vec3::new(0.0, 10.0, 5.0);
        let ray_dir = (Vec3::new(0.0, 0.0, 0.0) - camera_pos).normalize();

        let result = resolve_ray_to_ground(camera_pos, ray_dir, &empty_query, 100.0, 0.5);

        assert!(result.is_some(), "Ray should hit Y=0 plane as fallback");
        let ground_pos = result.unwrap();
        assert_eq!(ground_pos.y, 0.0, "Height should be 0.0 (fallback plane)");
    }

    #[test]
    fn test_resolve_ray_to_ground_no_hit() {
        let (query, _manifest) = create_test_surface_query();

        // Camera looking away from the surface
        let camera_pos = Vec3::new(0.0, 10.0, 5.0);
        let ray_dir = Vec3::new(0.0, 1.0, 0.0); // Looking straight up

        let result = resolve_ray_to_ground(camera_pos, ray_dir, &query, 100.0, 0.5);

        assert!(
            result.is_none(),
            "Ray looking up should not hit any surface"
        );
    }

    #[test]
    fn test_resolve_ray_to_ground_parallel_to_plane() {
        let empty_query = SurfaceQuery::from_manifest(&MapManifest {
            version: 2,
            map_id: "empty".to_string(),
            display_name: "Empty Map".to_string(),
            bounds: MapBounds {
                min_x: -10.0,
                max_x: 10.0,
                min_z: -10.0,
                max_z: 10.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        });

        // Ray parallel to Y=0 plane
        let camera_pos = Vec3::new(0.0, 0.0, 0.0);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0); // Horizontal ray

        let result = resolve_ray_to_ground(camera_pos, ray_dir, &empty_query, 100.0, 0.5);

        assert!(
            result.is_none(),
            "Horizontal ray should not intersect plane"
        );
    }

    #[test]
    fn test_resolve_ray_to_ground_zero_direction() {
        let (query, _manifest) = create_test_surface_query();

        let camera_pos = Vec3::new(0.0, 10.0, 5.0);
        let zero_dir = Vec3::ZERO;

        let result = resolve_ray_to_ground(camera_pos, zero_dir, &query, 100.0, 0.5);

        assert!(result.is_none(), "Zero direction should return None");
    }

    #[test]
    fn test_resolve_ray_to_ground_height_tolerance() {
        let (query, _manifest) = create_test_surface_query();

        // Camera positioned such that ray passes near but not through the surface
        let camera_pos = Vec3::new(0.0, 10.0, 5.0);
        let ray_dir = (Vec3::new(0.0, 2.5, 0.0) - camera_pos).normalize(); // Aiming slightly above surface

        let result = resolve_ray_to_ground(camera_pos, ray_dir, &query, 100.0, 0.5);

        // Should still find the surface within tolerance
        assert!(result.is_some(), "Ray should find surface within tolerance");
        let ground_pos = result.unwrap();
        assert_eq!(ground_pos.y, 2.0, "Height should be resolved to surface");
    }
}
