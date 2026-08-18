//! Cleave – Sword primary melee arc.
//!
//! Visual: **horizontal crescent** (wide flat box rotated to an arc angle)
//! sweeping in front of caster + particle-like small spheres along edge.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_box, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime};

const ARC_ANGLE_DEG: f32 = 90.0;
const ARC_RADIUS: f32 = 1.8;
const BLADE_THICKNESS: f32 = 0.06;
const SPARK_COUNT: usize = 6;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::SWORD;
    let base = effect.start + Vec3::Y * 0.8;

    // Crescent blade – wide flat box
    let arc_len = ARC_RADIUS * ARC_ANGLE_DEG.to_radians();
    spawn_box(
        commands,
        meshes,
        materials,
        base + Vec3::Z * (ARC_RADIUS * 0.5),
        Vec3::new(arc_len, BLADE_THICKNESS, 0.4),
        color,
        0.8,
        4.5,
        VfxLifetime::new(0.28),
    );

    // Sparks along the arc
    for i in 0..SPARK_COUNT {
        let frac = i as f32 / (SPARK_COUNT.saturating_sub(1)) as f32;
        let angle = (frac - 0.5) * ARC_ANGLE_DEG.to_radians();
        let rad = ARC_RADIUS * 0.8;
        let offset = Vec3::new(angle.sin() * rad, 0.0, angle.cos() * rad);

        spawn_sphere(
            commands,
            meshes,
            materials,
            base + offset,
            0.07,
            Color::WHITE,
            0.95,
            5.0,
            Vec3::splat(0.1 + frac * 0.15),
            VfxExpandFade::new(0.18, 0.05, 0.4),
        );
    }
}
