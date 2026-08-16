//! Client visual for `molten_eruption`.
//!
//! Six staggered fire circles in a ring around the caster.
//! Red warning cylinders pulse, then orange mini pillars erupt briefly.

use bevy::color::Color;
use bevy::prelude::*;
use bevymmo_shared::color::to_linear;

use bevymmo_shared::network::protocol::SpellVisualEffect;
use bevymmo_shared::spells_impl::dragon_enemy::molten_eruption::MoltenEruptionSpell;
use bevymmo_shared::spells_impl::dragon_enemy::{ASH_RED, FIRE_ORANGE};

use crate::spells::effects::SpellVisual;

const PILLAR_ERUPT_SECONDS: f32 = 0.4;

#[derive(Component)]
pub struct MoltenEruptionWarningVisual {
    elapsed_seconds: f32,
    duration_seconds: f32,
    circle_index: usize,
}

#[derive(Component)]
pub struct MoltenEruptionPillarVisual {
    elapsed_seconds: f32,
    circle_index: usize,
}

/// Spawns six warning circles and pillars in a ring around the caster.
///
/// Each circle has staggered timing (0.0-0.75s delays). The visual creates
/// ground warnings that pulse during their delay, then erupt into mini pillars.
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
    let caster_pos = effect.start;
    let _max_delay = MoltenEruptionSpell::impact_delay(5);

    let warning_mesh = meshes.add(Cylinder::new(MoltenEruptionSpell::CIRCLE_RADIUS, 0.05));
    let warning_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.1, 0.1, 0.45),
        emissive: to_linear(ASH_RED),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let pillar_mesh = meshes.add(Cylinder::new(MoltenEruptionSpell::CIRCLE_RADIUS, 2.5));
    let pillar_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.45, 0.05, 0.85),
        emissive: to_linear(FIRE_ORANGE),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // Spawn 6 circles in a ring
    for index in 0..MoltenEruptionSpell::CIRCLE_COUNT {
        let center = MoltenEruptionSpell::circle_center(index, caster_pos);
        let delay = MoltenEruptionSpell::impact_delay(index);
        let duration = delay + PILLAR_ERUPT_SECONDS;

        // Warning circle
        commands.spawn((
            Mesh3d(warning_mesh.clone()),
            MeshMaterial3d(warning_material.clone()),
            Transform::from_translation(center + Vec3::Y * 0.05),
            SpellVisual,
            MoltenEruptionWarningVisual {
                elapsed_seconds: 0.0,
                duration_seconds: duration,
                circle_index: index,
            },
        ));

        // Pillar (hidden initially)
        commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(pillar_material.clone()),
            Transform::from_translation(center).with_scale(Vec3::ZERO),
            SpellVisual,
            MoltenEruptionPillarVisual {
                elapsed_seconds: 0.0,
                circle_index: index,
            },
        ));
    }
}

/// Animates warning circles and erupting pillars.
///
/// Each circle follows its own staggered timing: pulse during delay,
/// then erupt. Warnings fade after their pillar erupts.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: ParamSet<(
        Query<(Entity, &mut Transform, &mut MoltenEruptionWarningVisual)>,
        Query<(Entity, &mut Transform, &mut MoltenEruptionPillarVisual)>,
    )>,
) {
    let delta = time.delta_secs();
    animate_warnings(delta, &mut commands, &mut visuals.p0());
    animate_pillars(delta, &mut commands, &mut visuals.p1());
}

fn animate_warnings(
    delta: f32,
    commands: &mut Commands,
    warnings: &mut Query<(Entity, &mut Transform, &mut MoltenEruptionWarningVisual)>,
) {
    for (entity, mut transform, mut visual) in warnings.iter_mut() {
        visual.elapsed_seconds += delta;
        let impact_time = MoltenEruptionSpell::impact_delay(visual.circle_index);

        if visual.elapsed_seconds >= visual.duration_seconds {
            commands.entity(entity).despawn();
            continue;
        }

        if visual.elapsed_seconds < impact_time {
            let pulse = 1.0 + (visual.elapsed_seconds * 8.0).sin() * 0.03;
            transform.scale = Vec3::new(pulse, 1.0, pulse);
        } else {
            let fade_progress =
                ((visual.elapsed_seconds - impact_time) / PILLAR_ERUPT_SECONDS).clamp(0.0, 1.0);
            let fade_scale = 1.0 + fade_progress * 0.15;
            transform.scale = Vec3::new(fade_scale, 1.0, fade_scale);
        }
    }
}

fn animate_pillars(
    delta: f32,
    commands: &mut Commands,
    pillars: &mut Query<(Entity, &mut Transform, &mut MoltenEruptionPillarVisual)>,
) {
    for (entity, mut transform, mut visual) in pillars.iter_mut() {
        visual.elapsed_seconds += delta;
        let erupt_start = MoltenEruptionSpell::impact_delay(visual.circle_index);
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
        transform.scale.y *= 1.0 - erupt_progress * 0.4;
    }
}
