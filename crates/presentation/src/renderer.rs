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
        app.add_observer(on_entity_position_added);
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

fn on_entity_position_added(
    trigger: On<Add, Position>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_assets: Option<Res<PlayerAssets>>,
    dragon_assets: Option<Res<BossDragonAssets>>,
    renderer_assets: Option<Res<RendererAssets>>,
    entities: Query<
        (
            &Position,
            &EntityColor,
            Option<&EntityKind>,
            Option<&ProjectileVisual>,
        ),
        Without<RenderedEntity>,
    >,
) {
    let entity = trigger.entity;
    let Ok((position, color, kind, projectile_visual)) = entities.get(entity) else {
        return;
    };

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
        let is_player = kind.map_or(false, |k| *k == EntityKind::Player);
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
        } else if kind.map_or(false, |k| *k == EntityKind::Hostile) {
            if let Some(assets) = dragon_assets.as_ref() {
                commands.entity(entity).insert((
                    WorldAssetRoot(assets.scene.clone()),
                    Transform::from_translation(position.0)
                        .with_scale(Vec3::splat(BOSS_DRAGON_SCENE_SCALE)),
                    RenderedEntity,
                ));
            }
        } else {
            let mesh = if let Some(ra) = renderer_assets.as_ref() {
                ra.fallback_mesh_small.clone()
            } else {
                meshes.add(Cuboid::new(2.0, 2.0, 2.0))
            };
            let material = materials.add(StandardMaterial {
                base_color: color.0,
                ..default()
            });
            commands.entity(entity).insert((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(position.0),
                RenderedEntity,
            ));
        }
    }
}

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
            let is_player = kind.map_or(false, |k| *k == EntityKind::Player);
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
            } else if kind.map_or(false, |k| *k == EntityKind::Hostile) {
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

fn sync_transforms(
    mut entities: Query<
        (&Position, Option<&LookDirection>, &mut Transform),
        Or<(Changed<Position>, Changed<LookDirection>)>,
    >,
) {
    for (position, look_direction, mut transform) in entities.iter_mut() {
        transform.translation = position.0;
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

/// Removes translations embedded in the instantiated player scene. Bevy may
/// place an intermediate scene entity between `WorldAssetRoot` and `Node0`, so
/// inspect the full parent chain instead of assuming a direct child.
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
            transform.translation = Vec3::ZERO;
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
