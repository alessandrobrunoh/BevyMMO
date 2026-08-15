//! Client visual for `searing_breath`.
//!
//! Fire cone impact visual spawned when the spell fires.
//! A cone primitive expands over 0.1s then fades over 0.5s, total 0.6s lifetime.

use bevymmo_shared::color::to_linear;
use bevy::color::Color;
use bevy::prelude::*;

use bevymmo_shared::network::protocol::SpellVisualEffect;
use bevymmo_shared::spells_impl::dragon_enemy::FIRE_ORANGE;

use crate::spells::effects::SpellVisual;

const CONE_SECONDS: f32 = 0.6;
const EXPAND_SECONDS: f32 = 0.1;
const FADE_SECONDS: f32 = 0.5;

#[derive(Component)]
pub struct SearingBreathConeVisual {
    elapsed_seconds: f32,
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Spawns a fire cone at the caster position facing the breath direction.
///
/// The cone expands from scale 0.3 to 1.0 over 0.1s, then fades over 0.5s.
/// The cone is rotated -90° on X to lay flat and point forward.
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
    let start = effect.start;
    let end = effect.end;

    // Calculate direction and rotation
    let direction = (end - start).normalize();
    let rotation = if direction.length() > 1e-6 {
        // Default cone points up (+Y), we need it to point forward
        // Rotate -90° on X to lay flat, then face the direction
        let flat_direction = Vec3::new(direction.x, 0.0, direction.z).normalize();
        Quat::from_rotation_y(-flat_direction.x.atan2(flat_direction.z))
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    } else {
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    };

    let mesh = meshes.add(Cone {
        radius: 4.0,
        height: 14.0,
    });
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.45, 0.05, 0.85),
        emissive: to_linear(FIRE_ORANGE),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(start)
            .with_rotation(rotation)
            .with_scale(Vec3::new(0.3, 1.0, 0.3)),
        SpellVisual,
        SearingBreathConeVisual {
            elapsed_seconds: 0.0,
        },
    ));
}

/// Animates the fire cone expansion and fade.
///
/// Phase 1 (0-0.1s): scale.z expands from 0.3 to 1.0 with ease_out_cubic.
/// Phase 2 (0.1-0.6s): fade out simulated by shrinking.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut cones: Query<(Entity, &mut Transform, &mut SearingBreathConeVisual)>,
) {
    let delta = time.delta_secs();

    for (entity, mut transform, mut visual) in cones.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = visual.elapsed_seconds;

        if t >= CONE_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }

        if t < EXPAND_SECONDS {
            // Expansion phase: scale.z 0.3 -> 1.0
            let progress = t / EXPAND_SECONDS;
            let expand = ease_out_cubic(progress);
            let scale = 0.3 + expand * 0.7;
            transform.scale = Vec3::new(scale, 1.0, scale);
        } else {
            // Fade phase: shrink scale
            let fade_progress = (t - EXPAND_SECONDS) / FADE_SECONDS;
            let fade = 1.0 - fade_progress;
            let scale = fade.max(0.0);
            transform.scale = Vec3::new(scale, 1.0, scale);
        }
    }
}
