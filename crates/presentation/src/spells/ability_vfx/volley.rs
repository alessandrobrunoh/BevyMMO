//! Volley – Bow secondary multi-projectile.
//!
//! Visual: **fan of small spheres** spreading in an arc from caster toward
//! impact area + wide ground disc.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_disc, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxPulseRing};

const ARROW_COUNT: usize = 5;
const SPREAD_ANGLE_DEG: f32 = 36.0; // total fan width
const CAST_HEIGHT: f32 = 1.15;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::BOW;
    let base_dir = (effect.end - effect.start).normalize_or_zero();
    let center = effect.end;

    // Fan of arrows (small spheres representing projectiles)
    for i in 0..ARROW_COUNT {
        let frac = i as f32 / (ARROW_COUNT.saturating_sub(1)) as f32; // 0..1
        let angle_offset =
            (frac - 0.5) * SPREAD_ANGLE_DEG.to_radians();
        // Rotate base_dir around Y
        let rot = Quat::from_rotation_y(angle_offset);
        let dir = rot * base_dir;
        let offset = dir * 1.2;

        spawn_sphere(
            commands,
            meshes,
            materials,
            effect.start + Vec3::Y * CAST_HEIGHT + offset,
            0.1,
            color,
            0.85,
            3.5,
            Vec3::splat(0.12 + frac * 0.08),
            VfxExpandFade::new(0.22, 0.08, 0.5),
        );
    }

    // Ground impact area – wide pulsing disc
    spawn_disc(
        commands,
        meshes,
        materials,
        center,
        2.0,
        0.04,
        color.with_hue(0.15), // slight yellow-green shift
        0.4,
        2.0,
        Vec3::splat(0.2),
        VfxPulseRing::new(0.15, 0.3),
    );
}
