//! Great Manifestation – Staff ultimate.
//!
//! Visual: **tall vertical pillar of stacked discs** (3 cylinders) that
//! grow upward like a summoning circle + outer rotating ring.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_disc, spawn_torus};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxSpinExpand};

const PILLAR_HEIGHTS: [f32; 3] = [0.05, 1.2, 2.4];
const PILLAR_RADII: [f32; 3] = [1.6, 1.2, 0.7];

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::STAFF;
    let base = effect.end;

    // Stacked pillar discs
    for i in 0..3 {
        spawn_disc(
            commands,
            meshes,
            materials,
            base + Vec3::Y * PILLAR_HEIGHTS[i],
            PILLAR_RADII[i],
            0.08,
            color,
            0.5 - (i as f32) * 0.12,
            3.0 - (i as f32) * 0.5,
            Vec3::splat(0.1),
            VfxExpandFade::new(0.7, 0.1, 1.4 + (i as f32) * 0.3),
        );
    }

    // Outer rotating binding ring
    spawn_torus(
        commands,
        meshes,
        materials,
        base + Vec3::Y * 0.1,
        2.0,
        0.07,
        Color::srgb(1.0, 0.85, 0.25), // gold accent
        0.45,
        3.5,
        VfxSpinExpand::new(0.9, 0.2, 2.2, 6.0),
    );
}
