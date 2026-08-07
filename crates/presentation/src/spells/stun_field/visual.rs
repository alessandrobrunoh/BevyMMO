//! Client-side visual for Stun Field.
//!
//! The server emits this visual after the instant cast. The client shows a
//! pulsing orange warning circle on the ground for 0.5 seconds, followed by
//! a brief expanding flash at impact.

use bevy::color::Color;
use bevy::prelude::*;

use bevymmo_shared::network::protocol::SpellVisualEffect;
use bevymmo_shared::spells_impl::stun_field::StunFieldSpell;

use crate::spells::effects::SpellVisual;

const IMPACT_FLASH_SECONDS: f32 = 0.3;

#[derive(Component)]
pub struct StunFieldWarningVisual {
    elapsed_seconds: f32,
    duration_seconds: f32,
}

#[derive(Component)]
pub struct StunFieldImpactFlash {
    elapsed_seconds: f32,
}

/// Spawns the warning circle and prepares the impact flash.
///
/// The warning circle pulses gently during the delay, then at t=0.5s a brief
/// expanding flash appears. Stun remains server-authoritative and is applied
/// by the AoE system after the configured delay.
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
    let duration = StunFieldSpell::IMPACT_DELAY_SECONDS + IMPACT_FLASH_SECONDS;

    let marker_mesh = meshes.add(Cylinder::new(StunFieldSpell::AREA_RADIUS, 0.05));
    let marker_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.55, 0.0, 0.45),
        emissive: LinearRgba::rgb(0.9, 0.45, 0.0),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Mesh3d(marker_mesh),
        MeshMaterial3d(marker_material),
        Transform::from_translation(center + Vec3::Y * 0.05),
        SpellVisual,
        StunFieldWarningVisual {
            elapsed_seconds: 0.0,
            duration_seconds: duration,
        },
    ));

    let flash_mesh = meshes.add(Cylinder::new(StunFieldSpell::AREA_RADIUS, 0.05));
    let flash_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.55, 0.0, 0.6),
        emissive: LinearRgba::rgb(0.9, 0.45, 0.0),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Mesh3d(flash_mesh),
        MeshMaterial3d(flash_material),
        Transform::from_translation(center + Vec3::Y * 0.05).with_scale(Vec3::ZERO),
        SpellVisual,
        StunFieldImpactFlash {
            elapsed_seconds: 0.0,
        },
    ));
}

/// Animates warning and flash visuals.
///
/// The warning circle pulses during the delay, then fades after impact.
/// The flash appears briefly at impact and expands before despawning.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
#[allow(clippy::type_complexity)]
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: ParamSet<(
        Query<(Entity, &mut Transform, &mut StunFieldWarningVisual)>,
        Query<(Entity, &mut Transform, &mut StunFieldImpactFlash)>,
    )>,
) {
    let delta = time.delta_secs();
    animate_warnings(delta, &mut commands, &mut visuals.p0());
    animate_flashes(delta, &mut commands, &mut visuals.p1());
}

fn animate_warnings(
    delta: f32,
    commands: &mut Commands,
    warnings: &mut Query<(Entity, &mut Transform, &mut StunFieldWarningVisual)>,
) {
    for (entity, mut transform, mut visual) in warnings.iter_mut() {
        visual.elapsed_seconds += delta;
        let impact_time = StunFieldSpell::IMPACT_DELAY_SECONDS;

        if visual.elapsed_seconds >= visual.duration_seconds {
            commands.entity(entity).despawn();
            continue;
        }

        if visual.elapsed_seconds < impact_time {
            let pulse = 1.0 + (visual.elapsed_seconds * 8.0).sin() * 0.03;
            transform.scale = Vec3::new(pulse, 1.0, pulse);
            continue;
        }

        let fade_progress =
            ((visual.elapsed_seconds - impact_time) / IMPACT_FLASH_SECONDS).clamp(0.0, 1.0);
        let fade_scale = 1.0 + fade_progress * 0.2;
        transform.scale = Vec3::new(fade_scale, 1.0, fade_scale);
    }
}

fn animate_flashes(
    delta: f32,
    commands: &mut Commands,
    flashes: &mut Query<(Entity, &mut Transform, &mut StunFieldImpactFlash)>,
) {
    for (entity, mut transform, mut visual) in flashes.iter_mut() {
        visual.elapsed_seconds += delta;
        let impact_start = StunFieldSpell::IMPACT_DELAY_SECONDS;
        let impact_end = impact_start + IMPACT_FLASH_SECONDS;

        if visual.elapsed_seconds >= impact_end {
            commands.entity(entity).despawn();
            continue;
        }

        if visual.elapsed_seconds < impact_start {
            transform.scale = Vec3::ZERO;
            continue;
        }

        let flash_progress =
            ((visual.elapsed_seconds - impact_start) / IMPACT_FLASH_SECONDS).clamp(0.0, 1.0);
        let flash_scale = 1.0 + flash_progress * 0.4;
        transform.scale = Vec3::new(flash_scale, 1.0, flash_scale);
    }
}
