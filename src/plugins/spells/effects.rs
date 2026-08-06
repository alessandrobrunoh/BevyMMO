//! Client-only visual spell effects.
//!
//! Questo modulo è volutamente isolato: per rimuovere le animazioni locali
//! delle spell basta togliere `client_effect_systems(app)` dal `SpellsPlugin`
//! e cancellare questo file.
//!
//! Gli effetti visivi arrivano dal server come messaggi `SpellVisualEffect`
//! replicati su tutti i client, garantendo che Player2 veda la fireball di Player1.

use bevy::prelude::*;

use crate::game_state::{GameScreen, Screen};
use crate::network::mode::has_client;
use crate::network::protocol::SpellVisualEffect;

const FIREBALL_DURATION_SECONDS: f32 = 0.35;
const FIREBALL_SIZE: f32 = 0.28;

#[derive(Component)]
struct FireballVisual {
    start: Vec3,
    end: Vec3,
    elapsed_seconds: f32,
    duration_seconds: f32,
}

pub fn client_effect_systems(app: &mut App) {
    app.add_systems(
        Update,
        (spawn_fireball_visuals, animate_fireball_visuals)
            .chain()
            .run_if(has_client)
            .run_if(in_gameplay),
    );
    app.add_systems(
        Update,
        cleanup_fireball_visuals
            .run_if(has_client)
            .run_if(not_in_gameplay),
    );
}

fn in_gameplay(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

fn not_in_gameplay(screen: Res<GameScreen>) -> bool {
    !in_gameplay(screen)
}

fn spawn_fireball_visuals(
    mut commands: Commands,
    mut messages: MessageReader<SpellVisualEffect>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for effect in messages.read() {
        let mesh = meshes.add(Cuboid::new(FIREBALL_SIZE, FIREBALL_SIZE, FIREBALL_SIZE));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.35, 0.05),
            emissive: LinearRgba::rgb(1.0, 0.25, 0.02),
            ..default()
        });

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(effect.start + Vec3::Y * 0.8),
            FireballVisual {
                start: effect.start + Vec3::Y * 0.8,
                end: effect.end + Vec3::Y * 0.8,
                elapsed_seconds: 0.0,
                duration_seconds: FIREBALL_DURATION_SECONDS,
            },
        ));
    }
}

fn animate_fireball_visuals(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: Query<(Entity, &mut Transform, &mut FireballVisual)>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, mut transform, mut visual) in visuals.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = (visual.elapsed_seconds / visual.duration_seconds).clamp(0.0, 1.0);
        transform.translation = visual.start.lerp(visual.end, t);
        transform.scale = Vec3::splat(1.0 + t * 0.6);

        if t >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn cleanup_fireball_visuals(mut commands: Commands, visuals: Query<Entity, With<FireballVisual>>) {
    for entity in visuals.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_visual_effect_stores_start_and_end_points() {
        let effect = SpellVisualEffect {
            spell_id: "fireball".to_string(),
            start: Vec3::ZERO,
            end: Vec3::Z,
        };
        assert_eq!(effect.start, Vec3::ZERO);
        assert_eq!(effect.end, Vec3::Z);
    }
}
