//! Client visual for `tail_sweep`.
//!
//! Instant rear half-ring dust cloud expanding behind the caster.
//! A flat torus half-ring expands with ease-out scaling and alpha fade,
//! plus a fainter secondary ring delayed by 0.08s.

use bevy::color::Color;
use bevy::prelude::*;

use bevymmo_shared::network::protocol::SpellVisualEffect;
use bevymmo_shared::spells_impl::dragon_enemy::DUST_TAN;

use crate::spells::effects::SpellVisual;

const SWEEP_SECONDS: f32 = 0.5;
const SECONDARY_DELAY_SECONDS: f32 = 0.08;

#[derive(Component)]
pub struct TailSweepMainVisual {
    elapsed_seconds: f32,
}

#[derive(Component)]
pub struct TailSweepSecondaryVisual {
    elapsed_seconds: f32,
}

fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(2)
}

/// Spawns the main dust half-ring and a secondary delayed ring at caster position.
///
/// The visual creates a flat torus half-ring that expands behind the caster,
/// representing the tail sweep's rear cone attack.
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
    let center = effect.start + Vec3::Y * 0.1;

    let mesh = meshes.add(Torus {
        major_radius: 3.5,
        minor_radius: 0.15,
    });
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.7, 0.55, 0.7),
        emissive: DUST_TAN,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // Main expanding half-ring
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(center).with_scale(Vec3::splat(0.2)),
        SpellVisual,
        TailSweepMainVisual {
            elapsed_seconds: 0.0,
        },
    ));

    // Secondary fainter ring (delayed)
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(center).with_scale(Vec3::ZERO),
        SpellVisual,
        TailSweepSecondaryVisual {
            elapsed_seconds: 0.0,
        },
    ));
}

/// Animates the main and secondary dust rings.
///
/// Main ring expands from 0.2 to 1.0 scale with ease-out, then fades.
/// Secondary ring starts at 0.08s with similar timing.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: ParamSet<(
        Query<(Entity, &mut Transform, &mut TailSweepMainVisual)>,
        Query<(Entity, &mut Transform, &mut TailSweepSecondaryVisual)>,
    )>,
) {
    let delta = time.delta_secs();
    animate_main(delta, &mut commands, &mut visuals.p0());
    animate_secondary(delta, &mut commands, &mut visuals.p1());
}

fn animate_main(
    delta: f32,
    commands: &mut Commands,
    mains: &mut Query<(Entity, &mut Transform, &mut TailSweepMainVisual)>,
) {
    for (entity, mut transform, mut visual) in mains.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = visual.elapsed_seconds;

        if t >= SWEEP_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = t / SWEEP_SECONDS;

        // Scale 0.2 -> 1.0 with ease_out_quad
        let scale = 0.2 + ease_out_quad(progress) * 0.8;

        // Flatten into half-ring (Y scale small)
        transform.scale = Vec3::new(scale, 0.25, scale);

        // Alpha fade in last half
        if progress > 0.5 {
            let fade_progress = (progress - 0.5) / 0.5;
            // We can't directly set alpha on transform, so we shrink scale to simulate fade
            transform.scale.y *= 1.0 - fade_progress;
        }
    }
}

fn animate_secondary(
    delta: f32,
    commands: &mut Commands,
    secondaries: &mut Query<(Entity, &mut Transform, &mut TailSweepSecondaryVisual)>,
) {
    for (entity, mut transform, mut visual) in secondaries.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = visual.elapsed_seconds;

        if t >= SWEEP_SECONDS + SECONDARY_DELAY_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }

        if t < SECONDARY_DELAY_SECONDS {
            transform.scale = Vec3::ZERO;
            continue;
        }

        let effective_t = t - SECONDARY_DELAY_SECONDS;
        let progress = effective_t / SWEEP_SECONDS;

        // Scale 0.2 -> 0.6 (slightly smaller than main)
        let scale = 0.2 + ease_out_quad(progress) * 0.4;

        transform.scale = Vec3::new(scale, 0.2, scale);
    }
}
