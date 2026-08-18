//! Arcane Wave – Staff secondary AoE.
//!
//! Visual: **expanding torus** ring that travels outward from impact + inner
//! sphere burst.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxSpinExpand};
use crate::spells::ability_vfx::{palette, spawn_sphere, spawn_torus};

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::STAFF;
    let center = effect.end;

    // Central burst
    spawn_sphere(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.4,
        0.6,
        color,
        0.75,
        3.5,
        Vec3::splat(0.2),
        VfxExpandFade::new(0.35, 0.2, 1.6),
    );

    // Expanding wave ring (torus)
    spawn_torus(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.08,
        1.2,  // ring radius
        0.06, // tube thickness
        color,
        0.55,
        2.5,
        VfxSpinExpand::new(0.5, 0.3, 1.8, 12.0),
    );
}
