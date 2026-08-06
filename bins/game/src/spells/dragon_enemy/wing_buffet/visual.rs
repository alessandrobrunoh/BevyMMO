//! Client visual for `wing_buffet`.
//!
//! Instant expanding ring shockwave from caster.
//! A flat torus expands from scale 0.5 to 10.0 with alpha fade,
//! plus a fainter secondary ring delayed by 0.08s scaled 1.2x.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;
use crate::spells::dragon_enemy::DUST_TAN;

const EXPANSION_SECONDS: f32 = 0.45;
const SECONDARY_DELAY_SECONDS: f32 = 0.08;

#[derive(Component)]
pub struct WingBuffetMainVisual {
    elapsed_seconds: f32,
}

#[derive(Component)]
pub struct WingBuffetSecondaryVisual {
    elapsed_seconds: f32,
}

/// Spawns the main expanding torus and a secondary delayed ring at caster position.
///
/// The visual creates an expanding shockwave representing the wing buffet's
/// outward force.
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
        major_radius: 5.0,
        minor_radius: 0.2,
    });
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.7, 0.55, 0.7),
        emissive: DUST_TAN,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // Main expanding ring
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(center).with_scale(Vec3::splat(0.5)),
        SpellVisual,
        WingBuffetMainVisual {
            elapsed_seconds: 0.0,
        },
    ));

    // Secondary fainter ring (delayed)
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(center).with_scale(Vec3::ZERO),
        SpellVisual,
        WingBuffetSecondaryVisual {
            elapsed_seconds: 0.0,
        },
    ));
}

/// Animates the main and secondary expanding rings.
///
/// Main ring expands from 0.5 to 10.0 scale with alpha fade.
/// Secondary ring starts at 0.08s with 1.2x scale.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: ParamSet<(
        Query<(Entity, &mut Transform, &mut WingBuffetMainVisual)>,
        Query<(Entity, &mut Transform, &mut WingBuffetSecondaryVisual)>,
    )>,
) {
    let delta = time.delta_secs();
    animate_main(delta, &mut commands, &mut visuals.p0());
    animate_secondary(delta, &mut commands, &mut visuals.p1());
}

fn animate_main(
    delta: f32,
    commands: &mut Commands,
    mains: &mut Query<(Entity, &mut Transform, &mut WingBuffetMainVisual)>,
) {
    for (entity, mut transform, mut visual) in mains.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = visual.elapsed_seconds;

        if t >= EXPANSION_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = t / EXPANSION_SECONDS;

        // Scale 0.5 -> 10.0
        let scale = 0.5 + progress * 9.5;

        // Flatten into ring (Y scale small)
        transform.scale = Vec3::new(scale, 0.3, scale);

        // Alpha fade via scale simulation
        transform.scale.y *= (1.0 - progress) * 0.7;
    }
}

fn animate_secondary(
    delta: f32,
    commands: &mut Commands,
    secondaries: &mut Query<(Entity, &mut Transform, &mut WingBuffetSecondaryVisual)>,
) {
    for (entity, mut transform, mut visual) in secondaries.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = visual.elapsed_seconds;

        if t >= EXPANSION_SECONDS + SECONDARY_DELAY_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }

        if t < SECONDARY_DELAY_SECONDS {
            transform.scale = Vec3::ZERO;
            continue;
        }

        let effective_t = t - SECONDARY_DELAY_SECONDS;
        let progress = effective_t / EXPANSION_SECONDS;

        // Scale 0.5 -> 12.0 (1.2x of main)
        let scale = 0.5 + progress * 11.5;

        transform.scale = Vec3::new(scale, 0.25, scale);

        // Alpha fade
        transform.scale.y *= (1.0 - progress) * 0.35;
    }
}
