//! Generic client visual for Eidolon casts.
//!
//! A single expanding/fading burst at the impact point, used as the fallback
//! for any `SpellVisualEffect` whose `spell_id` isn't one of the classic
//! spells' bespoke visuals (see the `unknown => ...` arm of
//! `dispatch_visual_effects`). Every Eidolon `BaseAbility`/Essenza combo
//! shares this one visual for now — bespoke per-gesture/per-Essenza visuals
//! are future work, not a blocker for the gesture actually being visible.

use bevy::color::Color;
use bevy::prelude::*;

use bevymmo_shared::network::protocol::SpellVisualEffect;

use crate::spells::effects::SpellVisual;

const BURST_SECONDS: f32 = 0.35;

#[derive(Component)]
pub struct EidolonImpactBurst {
    elapsed_seconds: f32,
}

pub fn spawn(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, effect: &SpellVisualEffect) {
    let center = effect.start;
    let mesh = meshes.add(Sphere::new(0.6));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.85, 0.5, 0.75),
        emissive: LinearRgba::rgb(1.0, 0.7, 0.3),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(center + Vec3::Y * 0.5).with_scale(Vec3::splat(0.1)),
        SpellVisual,
        EidolonImpactBurst { elapsed_seconds: 0.0 },
    ));
}

pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut bursts: Query<(Entity, &mut Transform, &mut EidolonImpactBurst)>,
) {
    let delta = time.delta_secs();
    for (entity, mut transform, mut burst) in bursts.iter_mut() {
        burst.elapsed_seconds += delta;
        if burst.elapsed_seconds >= BURST_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = (burst.elapsed_seconds / BURST_SECONDS).clamp(0.0, 1.0);
        let scale = 0.1 + progress * 1.4;
        transform.scale = Vec3::splat(scale);
    }
}
