use bevy::prelude::*;

use bevymmo_shared::network::protocol::*;
use bevymmo_shared::entity::components::{EntityKind, EntityState};
use std::time::Duration;

use crate::game_state::{GameScreen, Screen};
use crate::assets::PlayerAssets;

#[derive(Component)]
pub struct RenderedEntity;

#[derive(Component)]
pub struct AnimationIndices {
    pub idle: AnimationNodeIndex,
    pub walk: AnimationNodeIndex,
}

#[derive(Component)]
pub struct PlayerAnimation(pub Entity);

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_entity_meshes, sync_transforms, update_colors, setup_animation_players, handle_animations)
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

fn spawn_entity_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_assets: Option<Res<PlayerAssets>>,
    entities: Query<(Entity, &Position, &EntityColor, Option<&EntityKind>, Option<&ProjectileVisual>), Without<RenderedEntity>>,
) {
    for (entity, position, color, kind, projectile_visual) in entities.iter() {

        let is_projectile = projectile_visual.is_some();
        if is_projectile {
            let mesh = meshes.add(Cuboid::new(0.45, 0.45, 0.45));
            let material = materials.add(StandardMaterial {
                base_color: color.0,
                emissive: LinearRgba::rgb(0.1, 0.7, 1.0),
                ..default()
            });
            commands.entity(entity).insert((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(position.0),
                RenderedEntity,
            ));
        } else {
            let is_player = kind.map_or(false, |k| *k == EntityKind::Player);
            if is_player {
                if player_assets.is_some() {
                    commands.entity(entity).insert((
                        Transform::from_translation(position.0),
                        RenderedEntity,
                    ));
                }
                // Skip if not loaded yet
            } else {
                let mesh = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
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

fn setup_animation_players(
    mut commands: Commands,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    parent_query: Query<&ChildOf>,
    entity_state_query: Query<Entity, With<EntityState>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    player_assets: Option<Res<PlayerAssets>>,
) {
    let Some(player_assets) = player_assets else { return; };

    for (entity, mut player) in players.iter_mut() {
        let mut current = entity;
        let mut root = None;
        while let Ok(parent) = parent_query.get(current) {
            current = parent.0;
            if entity_state_query.contains(current) {
                root = Some(current);
                break;
            }
        }

        if let Some(root_entity) = root {
            let mut graph = AnimationGraph::new();
            let idle_idx = graph.add_clip(player_assets.idle.clone(), 1.0, graph.root);
            let walk_idx = graph.add_clip(player_assets.walk.clone(), 1.0, graph.root);
            let graph_handle = graphs.add(graph);

            let mut transitions = AnimationTransitions::new();
            transitions.play(&mut player, idle_idx, Duration::ZERO).repeat();

            commands.entity(entity).insert((
                AnimationGraphHandle(graph_handle),
                transitions,
                AnimationIndices { idle: idle_idx, walk: walk_idx }
            ));

            commands.entity(root_entity).insert(PlayerAnimation(entity));
        }
    }
}

fn handle_animations(
    entity_state_query: Query<(&EntityState, &PlayerAnimation), Changed<EntityState>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions, &AnimationIndices)>,
) {
    for (state, player_anim) in entity_state_query.iter() {
        if let Ok((mut player, mut transitions, indices)) = animation_players.get_mut(player_anim.0) {
            match state {
                EntityState::Idle | EntityState::Dead => {
                    transitions.play(&mut player, indices.idle, Duration::from_millis(250)).repeat();
                }
                EntityState::Moving => {
                    transitions.play(&mut player, indices.walk, Duration::from_millis(250)).repeat();
                }
            }
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
