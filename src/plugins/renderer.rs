use crate::game_state::{GameScreen, Screen};
use crate::network::protocol::*;
use bevy::prelude::*;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // I sistemi di presentazione girano solo mentre il giocatore vede il
        // mondo (`InGame`/`Paused`). In `Paused` non fermano simulazione/rete:
        // riguardano solo mesh/material/transform locali.
        app.add_systems(
            Update,
            (spawn_entity_meshes, sync_transforms, update_colors)
                .chain()
                .run_if(in_game_or_paused),
        )
        // Uscendo dal game i componenti render locali vengono rimossi; le
        // repliche gameplay (Position/EntityColor) restano e il renderer può
        // ricrearli al re-entry.
        .add_systems(Update, cleanup_entity_render.run_if(not_in_game));
    }
}

/// Condizione di esecuzione: visibile solo nelle schermate che mostrano il mondo.
fn in_game_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

/// Condizione di esecuzione: tutte le schermate non-mondo (menu/settings/connecting).
fn not_in_game(screen: Res<GameScreen>) -> bool {
    !matches!(screen.0, Screen::InGame | Screen::Paused)
}

/// Genera una mesh per le entità di gioco replicate non appena entrambi i
/// componenti dati di render sono disponibili.
/// `Position` ed `EntityColor` sono replicati; i componenti di render restano
/// locali al client.
fn spawn_entity_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    entities: Query<(Entity, &Position, &EntityColor), Without<Mesh3d>>,
) {
    for (entity, position, color) in entities.iter() {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color.0,
                ..default()
            })),
            Transform::from_translation(position.0),
        ));
    }
}

/// Sync Transform della mesh da `Position` (replicata via lightyear).
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

/// Aggiorna il colore della mesh quando `EntityColor` cambia.
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

/// Rimuove i componenti render locali (Mesh3d, MeshMaterial3d, Transform)
/// dalle entità di gioco quando non siamo più in `InGame`/`Paused`.
///
/// Le entità e i loro componenti replicati (`Position`, `EntityColor`, ...)
/// restano: il renderer li ricrea al re-entry grazie a `spawn_entity_meshes`.
fn cleanup_entity_render(mut commands: Commands, entities: Query<Entity, With<Mesh3d>>) {
    for entity in entities.iter() {
        commands
            .entity(entity)
            .remove::<Mesh3d>()
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .remove::<Transform>();
    }
}
