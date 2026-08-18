//! Field – Focus secondary ground AoE.
//!
//! Visual: **flat cylinder** (energy field) on the ground with a **torus**
//! border ring + vertical pillar at center.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_capsule, spawn_disc, spawn_torus};
use crate::spells::ability_vfx::lifecycle::{VfxOscillate, VfxSpinExpand};

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::FOCUS;
    let center = effect.end;

    // Ground field disc – slowly breathing
    spawn_disc(
        commands,
        meshes,
        materials,
        center,
        1.8,
        0.04,
        color,
        0.4,
        2.0,
        Vec3::splat(0.85),
        VfxOscillate::new(0.65, 0.85, 0.12, 3.0),
    );

    // Border torus
    spawn_torus(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.06,
        1.8,
        0.05,
        Color::srgb(0.55, 0.75, 1.0), // slightly lighter blue
        0.5,
        3.0,
        VfxSpinExpand::new(0.65, 0.9, 1.15, 2.5),
    );

    // Center vertical pillar (energy conduit)
    spawn_capsule(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.9,
        0.12,
        1.8,
        Color::srgb(0.8, 0.9, 1.0),
        0.55,
        4.0,
        VfxOscillate::new(0.6, 1.0, 0.08, 4.0),
    );
}
