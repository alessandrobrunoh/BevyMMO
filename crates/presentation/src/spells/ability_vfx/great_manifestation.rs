//! Great Manifestation – Staff ultimate.
//!
//! Visual: **tall vertical pillar of stacked discs** (3 cylinders) that
//! grow upward like a summoning circle + outer rotating ring.

use bevy::prelude::*;

use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxSpinExpand};
use crate::spells::ability_vfx::{
    palette, spawn_disc, spawn_matching_footprint, spawn_torus, AbilityVfxSpec,
};

const PILLAR_HEIGHTS: [f32; 3] = [0.05, 1.2, 2.4];
const PILLAR_RADIUS_FRACTIONS: [f32; 3] = [1.0, 0.72, 0.42];

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: &AbilityVfxSpec,
) {
    let color = palette::STAFF;
    let base = spec.impact();
    spawn_matching_footprint(commands, meshes, materials, spec, color);

    for i in 0..3 {
        spawn_disc(
            commands,
            meshes,
            materials,
            base + Vec3::Y * PILLAR_HEIGHTS[i],
            (spec.radius * PILLAR_RADIUS_FRACTIONS[i]).max(0.4),
            0.08,
            color,
            0.5 - (i as f32) * 0.12,
            3.0 - (i as f32) * 0.5,
            Vec3::splat(0.1),
            VfxExpandFade::new(0.7, 0.1, 1.4 + (i as f32) * 0.3),
        );
    }

    spawn_torus(
        commands,
        meshes,
        materials,
        base + Vec3::Y * 0.1,
        spec.radius.max(0.5),
        0.07,
        Color::srgb(1.0, 0.85, 0.25),
        0.45,
        3.5,
        VfxSpinExpand::new(0.9, 0.2, 1.0, 6.0),
    );
}
