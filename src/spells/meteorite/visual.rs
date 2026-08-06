//! Client-side visual for Meteorite.
//!
//! The server emits this visual only after the CastTime completes. The client
//! then shows a red warning circle for the impact delay, followed by a falling
//! meteor rock and a short impact burst.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;
use crate::spells::meteorite::MeteoriteSpell;

const ROCK_FALL_SECONDS: f32 = 0.6;
const IMPACT_BURST_SECONDS: f32 = 0.25;
const ROCK_START_HEIGHT: f32 = 8.0;
const ROCK_IMPACT_HEIGHT: f32 = 0.65;
const ROCK_SIZE: f32 = 0.75;

#[derive(Component)]
pub struct MeteoriteWarningVisual {
    elapsed_seconds: f32,
    duration_seconds: f32,
}

#[derive(Component)]
pub struct MeteoriteRockVisual {
    elapsed_seconds: f32,
    center: Vec3,
}

/// Spawns the warning circle and preloads the falling rock above the target.
///
/// The rock starts hidden above the impact point and becomes visible during the
/// final fall window. Damage remains server-authoritative and is applied by the
/// AoE system after the configured delay.
///
/// # Example
/// ```rust,ignore
/// visual::spawn(&mut commands, &mut meshes, &mut materials, &effect);
/// ```
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
        MeteoriteWarningVisual {
            elapsed_seconds: 0.0,
            duration_seconds: duration,
        },
    ));

    let rock_mesh = meshes.add(Sphere::new(ROCK_SIZE));
    let rock_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.12, 0.08),
        emissive: LinearRgba::rgb(0.75, 0.18, 0.04),
        ..default()
    });

    commands.spawn((
        Mesh3d(rock_mesh),
        MeshMaterial3d(rock_material),
        Transform::from_translation(center + Vec3::Y * ROCK_START_HEIGHT).with_scale(Vec3::ZERO),
        SpellVisual,
        MeteoriteRockVisual {
            elapsed_seconds: 0.0,
            center,
        },
    ));
}

/// Animates warning and falling rock visuals.
///
/// The warning circle pulses during the delay, then expands briefly at impact.
/// The rock becomes visible only during the final fall window.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: ParamSet<(
        Query<(Entity, &mut Transform, &mut MeteoriteWarningVisual)>,
        Query<(Entity, &mut Transform, &mut MeteoriteRockVisual)>,
    )>,
) {
    let delta = time.delta_secs();
    animate_warnings(delta, &mut commands, &mut visuals.p0());
    animate_rocks(delta, &mut commands, &mut visuals.p1());
}

fn animate_warnings(
    delta: f32,
    commands: &mut Commands,
    warnings: &mut Query<(Entity, &mut Transform, &mut MeteoriteWarningVisual)>,
) {
    for (entity, mut transform, mut visual) in warnings.iter_mut() {
        visual.elapsed_seconds += delta;
        let impact_time = MeteoriteSpell::IMPACT_DELAY_SECONDS;

        if visual.elapsed_seconds >= visual.duration_seconds {
            commands.entity(entity).despawn();
            continue;
        }

        if visual.elapsed_seconds < impact_time {
            let pulse = 1.0 + (visual.elapsed_seconds * 6.0).sin() * 0.04;
            transform.scale = Vec3::new(pulse, 1.0, pulse);
            continue;
        }

        let burst_progress =
            ((visual.elapsed_seconds - impact_time) / IMPACT_BURST_SECONDS).clamp(0.0, 1.0);
        let burst_scale = 1.0 + burst_progress * 0.55;
        transform.scale = Vec3::new(burst_scale, 1.0, burst_scale);
    }
}

fn animate_rocks(
    delta: f32,
    commands: &mut Commands,
    rocks: &mut Query<(Entity, &mut Transform, &mut MeteoriteRockVisual)>,
) {
    for (entity, mut transform, mut visual) in rocks.iter_mut() {
        visual.elapsed_seconds += delta;
        let fall_start = MeteoriteSpell::IMPACT_DELAY_SECONDS - ROCK_FALL_SECONDS;
        let impact_end = MeteoriteSpell::IMPACT_DELAY_SECONDS + IMPACT_BURST_SECONDS;

        if visual.elapsed_seconds >= impact_end {
            commands.entity(entity).despawn();
            continue;
        }

        if visual.elapsed_seconds < fall_start {
            transform.scale = Vec3::ZERO;
            continue;
        }

        let fall_progress =
            ((visual.elapsed_seconds - fall_start) / ROCK_FALL_SECONDS).clamp(0.0, 1.0);
        let height = ROCK_START_HEIGHT.lerp(ROCK_IMPACT_HEIGHT, fall_progress);
        transform.translation = visual.center + Vec3::Y * height;
        transform.scale = Vec3::splat(1.0 + fall_progress * 0.25);
    }
}
