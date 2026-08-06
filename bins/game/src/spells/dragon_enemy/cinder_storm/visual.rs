//! Client visual for `cinder_storm`.
//!
//! Two delayed fire circles with ground warnings and erupting pillars.
//! Red warning cylinders pulse for 1.5s, then orange fire pillars erupt for 0.5s.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;
use crate::spells::dragon_enemy::cinder_storm::CinderStormSpell;
use crate::spells::dragon_enemy::{ASH_RED, FIRE_ORANGE};

const PILLAR_ERUPT_SECONDS: f32 = 0.5;
const SECOND_CIRCLE_OFFSET: f32 = 2.0;

#[derive(Component)]
pub struct CinderStormWarningVisual {
    elapsed_seconds: f32,
    duration_seconds: f32,
}

#[derive(Component)]
pub struct CinderStormPillarVisual {
    elapsed_seconds: f32,
}

/// Spawns two warning circles and prepares fire pillars at offset positions.
///
/// The visual creates two ground warnings that pulse during the impact delay,
/// then erupt into fire pillars. Both circles are offset by 2.0 units.
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
    let centroid = effect.start;
    let duration = CinderStormSpell::IMPACT_DELAY_SECONDS + PILLAR_ERUPT_SECONDS;

    let warning_mesh = meshes.add(Cylinder::new(CinderStormSpell::AREA_RADIUS, 0.05));
    let warning_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.1, 0.1, 0.45),
        emissive: ASH_RED,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let pillar_mesh = meshes.add(Cylinder::new(CinderStormSpell::AREA_RADIUS, 4.0));
    let pillar_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.45, 0.05, 0.85),
        emissive: FIRE_ORANGE,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // First circle at centroid
    let center1 = centroid;
    commands.spawn((
        Mesh3d(warning_mesh.clone()),
        MeshMaterial3d(warning_material.clone()),
        Transform::from_translation(center1 + Vec3::Y * 0.05),
        SpellVisual,
        CinderStormWarningVisual {
            elapsed_seconds: 0.0,
            duration_seconds: duration,
        },
    ));

    // Second circle offset by 2.0 on X axis
    let center2 = centroid + Vec3::X * SECOND_CIRCLE_OFFSET;
    commands.spawn((
        Mesh3d(warning_mesh.clone()),
        MeshMaterial3d(warning_material.clone()),
        Transform::from_translation(center2 + Vec3::Y * 0.05),
        SpellVisual,
        CinderStormWarningVisual {
            elapsed_seconds: 0.0,
            duration_seconds: duration,
        },
    ));

    // First pillar (hidden initially)
    commands.spawn((
        Mesh3d(pillar_mesh.clone()),
        MeshMaterial3d(pillar_material.clone()),
        Transform::from_translation(center1).with_scale(Vec3::ZERO),
        SpellVisual,
        CinderStormPillarVisual {
            elapsed_seconds: 0.0,
        },
    ));

    // Second pillar (hidden initially)
    commands.spawn((
        Mesh3d(pillar_mesh),
        MeshMaterial3d(pillar_material),
        Transform::from_translation(center2).with_scale(Vec3::ZERO),
        SpellVisual,
        CinderStormPillarVisual {
            elapsed_seconds: 0.0,
        },
    ));
}

/// Animates warning circles and erupting fire pillars.
///
/// Warnings pulse during the delay, then fade as pillars erupt.
/// Pillars grow from 0 to full height over 0.5s.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: ParamSet<(
        Query<(Entity, &mut Transform, &mut CinderStormWarningVisual)>,
        Query<(Entity, &mut Transform, &mut CinderStormPillarVisual)>,
    )>,
) {
    let delta = time.delta_secs();
    animate_warnings(delta, &mut commands, &mut visuals.p0());
    animate_pillars(delta, &mut commands, &mut visuals.p1());
}

fn animate_warnings(
    delta: f32,
    commands: &mut Commands,
    warnings: &mut Query<(Entity, &mut Transform, &mut CinderStormWarningVisual)>,
) {
    for (entity, mut transform, mut visual) in warnings.iter_mut() {
        visual.elapsed_seconds += delta;
        let impact_time = CinderStormSpell::IMPACT_DELAY_SECONDS;

        if visual.elapsed_seconds >= visual.duration_seconds {
            commands.entity(entity).despawn();
            continue;
        }

        if visual.elapsed_seconds < impact_time {
            let pulse = 1.0 + (visual.elapsed_seconds * 6.0).sin() * 0.04;
            transform.scale = Vec3::new(pulse, 1.0, pulse);
        } else {
            let fade_progress =
                ((visual.elapsed_seconds - impact_time) / PILLAR_ERUPT_SECONDS).clamp(0.0, 1.0);
            let fade_scale = 1.0 + fade_progress * 0.2;
            transform.scale = Vec3::new(fade_scale, 1.0, fade_scale);
        }
    }
}

fn animate_pillars(
    delta: f32,
    commands: &mut Commands,
    pillars: &mut Query<(Entity, &mut Transform, &mut CinderStormPillarVisual)>,
) {
    for (entity, mut transform, mut visual) in pillars.iter_mut() {
        visual.elapsed_seconds += delta;
        let erupt_start = CinderStormSpell::IMPACT_DELAY_SECONDS;
        let erupt_end = erupt_start + PILLAR_ERUPT_SECONDS;

        if visual.elapsed_seconds >= erupt_end {
            commands.entity(entity).despawn();
            continue;
        }

        if visual.elapsed_seconds < erupt_start {
            transform.scale = Vec3::ZERO;
            continue;
        }

        let erupt_progress =
            ((visual.elapsed_seconds - erupt_start) / PILLAR_ERUPT_SECONDS).clamp(0.0, 1.0);
        let scale = erupt_progress;
        transform.scale = Vec3::new(scale, scale, scale);

        // Fade out alpha via scale reduction
        transform.scale.y *= 1.0 - erupt_progress * 0.3;
    }
}
