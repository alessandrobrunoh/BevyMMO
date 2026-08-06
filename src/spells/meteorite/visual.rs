//! Visual client-side per la spell Meteorite.
//!
//! Il dispatcher in `plugins::spells::effects` chiama `spawn` quando arriva un
//! `SpellVisualEffect` con l'id di Meteorite. Il visual si autotima sulla base
//! delle costanti di `MeteoriteSpell`: cerchio di warning → caduta meteorite
//! negli ultimi `ROCK_FALL_SECONDS` → burst esplosivo.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;
use crate::spells::meteorite::MeteoriteSpell;

const ROCK_FALL_SECONDS: f32 = 0.6;
const IMPACT_BURST_SECONDS: f32 = 0.25;

#[derive(Component)]
pub struct MeteoriteVisual {
    elapsed_seconds: f32,
    /// Durata totale del visual: delay + burst.
    duration_seconds: f32,
    center: Vec3,
}

/// Spawn del marker di warning (cylinder rosso piatto). Il resto dell'animazione
/// (caduta + burst) è gestito in `animate` modificando questo stesso entity.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let center = effect.start;
    let duration = MeteoriteSpell::IMPACT_DELAY_SECONDS + IMPACT_BURST_SECONDS;

    let marker_mesh = meshes.add(Cylinder::new(MeteoriteSpell::AREA_RADIUS, 0.05));
    let marker_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.1, 0.1, 0.45),
        emissive: LinearRgba::rgb(0.6, 0.05, 0.05),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Mesh3d(marker_mesh),
        MeshMaterial3d(marker_material),
        Transform::from_translation(center + Vec3::Y * 0.05),
        SpellVisual,
        MeteoriteVisual {
            elapsed_seconds: 0.0,
            duration_seconds: duration,
            center,
        },
    ));
}

/// Anima il marker: pulsazione durante il delay, espansione durante la caduta
/// del meteorite, burst finale.
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: Query<(Entity, &mut Transform, &mut MeteoriteVisual)>,
) {
    let delta = time.delta_secs();
    for (entity, mut transform, mut visual) in visuals.iter_mut() {
        visual.elapsed_seconds += delta;

        let t = visual.elapsed_seconds;
        let impact_time = MeteoriteSpell::IMPACT_DELAY_SECONDS;
        let total = visual.duration_seconds;

        if t >= total {
            commands.entity(entity).despawn();
            continue;
        }

        if t < impact_time {
            // Warning: pulsazione lieve del cerchio.
            let pulse = 1.0 + (t * 6.0).sin() * 0.04;
            transform.scale = Vec3::new(pulse, 1.0, pulse);
        } else if t < impact_time + ROCK_FALL_SECONDS {
            // Caduta: il cerchio si espande progressivamente verso l'impatto.
            let progress = ((t - impact_time) / ROCK_FALL_SECONDS).clamp(0.0, 1.0);
            transform.scale = Vec3::new(
                1.0 + progress * 0.4,
                1.0 + progress * 4.0,
                1.0 + progress * 0.4,
            );
        } else {
            // Burst: rapida espansione finale prima del despawn.
            let burst_progress =
                ((t - impact_time - ROCK_FALL_SECONDS) / IMPACT_BURST_SECONDS).clamp(0.0, 1.0);
            transform.scale =
                Vec3::new(1.4 + burst_progress * 1.6, 1.0, 1.4 + burst_progress * 1.6);
        }
    }
}
