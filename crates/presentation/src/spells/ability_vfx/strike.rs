//! Strike – Gauntlets primary melee.
//!
//! Visual: **short thick capsule** (fist shape) at impact point + radial
//! shockwave boxes emanating outward (knockback visual).

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_box, spawn_capsule, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime};

const SHOCKWAVE_COUNT: usize = 4;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::GAUNTLETS;
    let target = effect.end + Vec3::Y * 0.75;

    // Fist impact capsule
    spawn_capsule(
        commands,
        meshes,
        materials,
        target,
        0.22,
        0.5,
        color,
        0.85,
        4.5,
        VfxLifetime::new(0.18),
    );

    // Impact flash
    spawn_sphere(
        commands,
        meshes,
        materials,
        target,
        0.28,
        Color::srgb(1.0, 0.95, 0.85),
        0.92,
        5.0,
        Vec3::splat(0.12),
        VfxExpandFade::new(0.14, 0.12, 0.7),
    );

    // Radial shockwave lines (boxes pointing outward)
    let dir = (effect.end - effect.start).normalize_or_zero();
    let right = dir.cross(Vec3::Y).normalize_or_zero();

    for i in 0..SHOCKWAVE_COUNT {
        let angle = (i as f32 / SHOCKWAVE_COUNT as f32) * std::f32::consts::TAU;
        let out_dir = (dir * angle.cos() + right * angle.sin()).normalize_or_zero();
        let sw_pos = target + out_dir * 0.6;

        spawn_box(
            commands,
            meshes,
            materials,
            sw_pos,
            Vec3::new(0.06, 0.06, 0.5),
            color.with_hue(i as f32 * 0.03),
            0.5,
            2.5,
            VfxExpandFade::new(0.2, 0.08, 0.6),
        );
    }
}
