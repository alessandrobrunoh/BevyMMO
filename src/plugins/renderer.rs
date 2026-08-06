use crate::game_state::{GameScreen, Screen};
use crate::network::protocol::*;
use bevy::prelude::*;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // Presentation systems run only while the player sees the world
        // (`InGame`/`Paused`). In `Paused` they do not stop simulation/network:
        // they only affect local mesh/material/transform components.
        app.add_systems(
            Update,
            (spawn_entity_meshes, sync_transforms, update_colors)
                .chain()
                .run_if(in_game_or_paused),
        )
        // Leaving the game removes local render components;
        // gameplay replicas (Position/EntityColor) remain and the renderer can
        // recreate them upon re-entry.
        .add_systems(Update, cleanup_entity_render.run_if(not_in_game));
    }
}

/// Run condition: visible only on screens that display the world.
fn in_game_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

/// Run condition: all non-world screens (menu/settings/connecting).
fn not_in_game(screen: Res<GameScreen>) -> bool {
    !matches!(screen.0, Screen::InGame | Screen::Paused)
}

/// Spawns a mesh for replicated game entities as soon as both render data
/// components are available.
/// `Position` and `EntityColor` are replicated; render components remain
/// local to the client.
fn spawn_entity_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    entities: Query<(Entity, &Position, &EntityColor, Option<&ProjectileVisual>), Without<Mesh3d>>,
) {
    for (entity, position, color, projectile_visual) in entities.iter() {
        let is_projectile = projectile_visual.is_some();
        let mesh = if is_projectile {
            meshes.add(Cuboid::new(0.45, 0.45, 0.45))
        } else {
            meshes.add(Cuboid::new(2.0, 2.0, 2.0))
        };
        let material = if is_projectile {
            materials.add(StandardMaterial {
                base_color: color.0,
                emissive: LinearRgba::rgb(0.1, 0.7, 1.0),
                ..default()
            })
        } else {
            materials.add(StandardMaterial {
                base_color: color.0,
                ..default()
            })
        };

        commands.entity(entity).insert((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position.0),
        ));
    }
}

/// Syncs mesh Transform from `Position` (replicated via lightyear).
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

/// Updates mesh color when `EntityColor` changes.
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

/// Removes local render components (Mesh3d, MeshMaterial3d, Transform)
/// from game entities when no longer in `InGame`/`Paused`.
///
/// Entities and their replicated components (`Position`, `EntityColor`, ...)
/// remain: the renderer recreates them upon re-entry thanks to `spawn_entity_meshes`.
fn cleanup_entity_render(mut commands: Commands, entities: Query<Entity, With<Mesh3d>>) {
    for entity in entities.iter() {
        commands
            .entity(entity)
            .remove::<Mesh3d>()
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .remove::<Transform>();
    }
}

