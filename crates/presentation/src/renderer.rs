use bevy::prelude::*;

use crate::assets::{BossDragonAssets, PlayerAssets};
use crate::game_state::{GameScreen, Screen};
use bevymmo_shared::entity::components::EntityKind;
use bevymmo_shared::network::protocol::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct RendererAssets {
    projectile_mesh: Handle<Mesh>,
    projectile_material: Handle<StandardMaterial>,
    fallback_mesh_small: Handle<Mesh>,
    color_materials: HashMap<[u32; 3], Handle<StandardMaterial>>,
}

impl RendererAssets {
    fn get_or_create_color_material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        color: Color,
    ) -> Handle<StandardMaterial> {
        let [r, g, b, _] = color.to_srgba().to_f32_array();
        let key = [r.to_bits(), g.to_bits(), b.to_bits()];
        self.color_materials
            .entry(key)
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: color,
                    ..default()
                })
            })
            .clone()
    }
}

fn init_renderer_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(RendererAssets {
        projectile_mesh: meshes.add(Cuboid::new(0.45, 0.45, 0.45)),
        projectile_material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            emissive: LinearRgba::rgb(0.1, 0.7, 1.0),
            ..default()
        }),
        fallback_mesh_small: meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
        color_materials: HashMap::new(),
    });
}

#[derive(Component)]
pub struct RenderedEntity;

/// Marks the scene root of a player model whose imported root node needs to be
/// anchored to the replicated gameplay position.
#[derive(Component)]
struct PlayerModelRoot;

/// Prevents re-normalizing the imported node once its scene is instantiated.
#[derive(Component)]
struct PlayerModelAnchored;

// The current player.glb is authored in game-world units (unlike the old
// oversized animated asset), so it must not be scaled down by 0.035.
const PLAYER_SCENE_SCALE: f32 = 1.0;
const BOSS_DRAGON_SCENE_SCALE: f32 = 0.12;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_renderer_assets);
        // Note: there is deliberately no `Add<Position>` observer here.
        // An observer and `spawn_entity_meshes` used to run the same branching
        // logic side by side. The observer fired before `PlayerAssets` /
        // `BossDragonAssets` finished loading, silently dropped those entities
        // (its `if let Some(assets)` arms have no `else`), and bypassed the
        // material cache — so whichever path won the race decided whether a
        // material was leaked per entity. `spawn_entity_meshes` is the retry
        // loop that already handled the not-yet-loaded case correctly, so it is
        // the single source of truth.
        app.add_systems(
            Update,
            (
                spawn_entity_meshes,
                sync_transforms,
                anchor_player_model,
                update_colors,
            )
                .chain()
                .run_if(in_game_or_paused),
        )
        .add_systems(Update, cleanup_entity_render.run_if(not_in_game));
    }
}

fn in_game_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

fn not_in_game(screen: Res<GameScreen>) -> bool {
    !matches!(screen.0, Screen::InGame | Screen::Paused)
}

/// Gives every replicated entity its local render components.
///
/// Runs every frame rather than as an `Add<Position>` observer on purpose: an
/// entity can be replicated before its glTF collection has finished loading,
/// and this retries until the assets exist.
fn spawn_entity_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_assets: Option<Res<PlayerAssets>>,
    dragon_assets: Option<Res<BossDragonAssets>>,
    mut renderer_assets: Option<ResMut<RendererAssets>>,
    entities: Query<
        (
            Entity,
            &Position,
            &EntityColor,
            Option<&EntityKind>,
            Option<&ProjectileVisual>,
        ),
        Without<RenderedEntity>,
    >,
) {
    for (entity, position, color, kind, projectile_visual) in entities.iter() {
        let is_projectile = projectile_visual.is_some();
        if is_projectile {
            let (mesh, material) = if let Some(ra) = renderer_assets.as_ref() {
                (ra.projectile_mesh.clone(), ra.projectile_material.clone())
            } else {
                (
                    meshes.add(Cuboid::new(0.45, 0.45, 0.45)),
                    materials.add(StandardMaterial {
                        base_color: color.0,
                        emissive: LinearRgba::rgb(0.1, 0.7, 1.0),
                        ..default()
                    }),
                )
            };
            commands.entity(entity).insert((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(position.0),
                RenderedEntity,
            ));
        } else {
            let is_player = kind.is_some_and(|k| *k == EntityKind::Player);
            if is_player {
                if let Some(assets) = player_assets.as_ref() {
                    commands.entity(entity).insert((
                        WorldAssetRoot(assets.scene.clone()),
                        Transform::from_translation(position.0)
                            .with_scale(Vec3::splat(PLAYER_SCENE_SCALE)),
                        PlayerModelRoot,
                        RenderedEntity,
                    ));
                }
            } else if kind.is_some_and(|k| *k == EntityKind::Hostile) {
                if let Some(assets) = dragon_assets.as_ref() {
                    commands.entity(entity).insert((
                        WorldAssetRoot(assets.scene.clone()),
                        Transform::from_translation(position.0)
                            .with_scale(Vec3::splat(BOSS_DRAGON_SCENE_SCALE)),
                        RenderedEntity,
                    ));
                }
            } else {
                let (mesh, material) = if let Some(ra) = renderer_assets.as_mut() {
                    (
                        ra.fallback_mesh_small.clone(),
                        ra.get_or_create_color_material(&mut materials, color.0),
                    )
                } else {
                    (
                        meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
                        materials.add(StandardMaterial {
                            base_color: color.0,
                            ..default()
                        }),
                    )
                };
                commands.entity(entity).insert((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(position.0),
                    RenderedEntity,
                ));
            }
        }
    }
}

/// Distance beyond which a position change is a teleport, not movement.
///
/// Comfortably above one tick of travel (a fast player covers ~0.3 m per tick)
/// and well below a respawn or a knockback, which must snap rather than glide
/// across the map.
const TELEPORT_SNAP_DISTANCE: f32 = 5.0;

/// How quickly the rendered transform catches up to `Position`, per second.
///
/// This is an exponential follow, not an interpolation between fixed steps.
/// The obvious approach — keep the previous and current fixed-step positions
/// and blend with `Time<Fixed>::overstep_fraction()` — assumes Bevy's fixed
/// schedule is what advances `Position`. It is not: Lightyear's prediction
/// rolls back and re-simulates several ticks inside a single frame, so the
/// recorded pair stops matching the step being blended and the result stutters
/// worse than no smoothing at all. Chasing the current value instead makes no
/// assumption about *when* `Position` was written.
///
/// At 25/s the transform closes ~92% of the gap in 100 ms: fast enough to feel
/// direct, slow enough to absorb an uneven tick.
const RENDER_FOLLOW_RATE: f32 = 25.0;

fn sync_transforms(
    time: Res<Time>,
    mut entities: Query<(&Position, Option<&LookDirection>, &mut Transform)>,
) {
    // Frame-rate independent exponential decay: the fraction of the remaining
    // gap closed this frame depends only on elapsed time, so the motion looks
    // identical at 60 and 240 Hz.
    let blend = 1.0 - (-RENDER_FOLLOW_RATE * time.delta_secs()).exp();

    for (position, look_direction, mut transform) in entities.iter_mut() {
        let target = position.0;
        transform.translation = if transform.translation.distance(target) > TELEPORT_SNAP_DISTANCE {
            target
        } else {
            transform.translation.lerp(target, blend.clamp(0.0, 1.0))
        };
        if let Some(look_direction) = look_direction {
            let direction = Vec3::new(look_direction.x, 0.0, look_direction.z);
            if direction.length_squared() > 0.001 {
                transform.rotation = Transform::default()
                    .looking_to(direction.normalize(), Vec3::Y)
                    .rotation;
            }
        }
    }
}

/// Removes horizontal offsets embedded in the instantiated player scene while
/// preserving its authored vertical placement. Bevy may place an intermediate
/// scene entity between `WorldAssetRoot` and `Node0`, so inspect the full parent
/// chain instead of assuming a direct child.
fn anchor_player_model(
    mut commands: Commands,
    roots: Query<Entity, With<PlayerModelRoot>>,
    parents: Query<&ChildOf>,
    mut scene_nodes: Query<
        (Entity, &mut Transform),
        (Without<PlayerModelRoot>, Without<PlayerModelAnchored>),
    >,
) {
    for (entity, mut transform) in &mut scene_nodes {
        if roots
            .iter()
            .any(|root| is_descendant_of(entity, root, &parents))
        {
            transform.translation.x = 0.0;
            transform.translation.z = 0.0;
            commands.entity(entity).insert(PlayerModelAnchored);
        }
    }
}

fn is_descendant_of(entity: Entity, root: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        if parent.0 == root {
            return true;
        }
        current = parent.0;
    }
    false
}

fn update_colors(
    entities: Query<(&EntityColor, &MeshMaterial3d<StandardMaterial>), Changed<EntityColor>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (color, handle) in entities.iter() {
        if let Some(mut mat) = materials.get_mut(&handle.0) {
            mat.base_color = color.0;
        }
    }
}

fn cleanup_entity_render(mut commands: Commands, entities: Query<Entity, With<RenderedEntity>>) {
    for entity in entities.iter() {
        commands
            .entity(entity)
            .remove::<RenderedEntity>()
            .remove::<Mesh3d>()
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .remove::<WorldAssetRoot>()
            .remove::<Transform>();
    }
}
