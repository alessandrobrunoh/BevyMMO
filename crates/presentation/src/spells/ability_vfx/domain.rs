//! Domain – Focus ultimate.
//!
//! Visual: **large hemisphere** of stacked tori (rings at ascending heights)
//! forming a dome-like barrier + central floating orb above.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_sphere, spawn_torus};
use crate::spells::ability_vfx::lifecycle::{VfxOscillate, VfxSpinExpand};

const RING_COUNT: usize = 4;
const MAX_DOME_RADIUS: f32 = 2.8;
const DOME_HEIGHT_STEP: f32 = 0.9;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::FOCUS;
    let base = effect.end;

    // Stacked dome rings (decreasing radius as they go up)
    for i in 0..RING_COUNT {
        let t = (i + 1) as f32 / RING_COUNT as f32;       // 0.25 .. 1.0
        let radius = MAX_DOME_RADIUS * (1.0 - t * 0.6);   // shrink toward top
        let height = DOME_HEIGHT_STEP * (i as f32 + 1.0);

        spawn_torus(
            commands,
            meshes,
            materials,
            base + Vec3::Y * height,
            radius,
            0.05,
            color.with_hue(-t * 0.05), // shift hue toward purple at top
            0.35 - t * 0.08,
            2.5 + t * 1.5,
            VfxSpinExpand::new(
                0.8 + t * 0.15,
                0.8 - t * 0.15,
                1.2 + t * 0.3,
                3.0 - t * 1.5 + i as f32 * 0.4,
            ),
        );
    }

    // Floating apex orb
    spawn_sphere(
        commands,
        meshes,
        materials,
        base + Vec3::Y * (DOME_HEIGHT_STEP * RING_COUNT as f32 + 0.5),
        0.4,
        Color::srgb(0.6, 0.8, 1.0),
        0.7,
        4.5,
        Vec3::splat(0.6),
        VfxOscillate::new(1.0, 0.7, 0.15, 2.0),
    );
}
