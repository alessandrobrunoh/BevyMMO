//! Arcane Wave – Staff secondary AoE.
//!
//! Visual: **expanding torus** ring that travels outward from impact + inner
//! sphere burst.

use bevy::prelude::*;

use crate::spells::ability_vfx::lifecycle::VfxExpandFade;
use crate::spells::ability_vfx::{palette, spawn_matching_footprint, spawn_sphere, AbilityVfxSpec};

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: &AbilityVfxSpec,
) {
    let color = palette::STAFF;
    spawn_matching_footprint(commands, meshes, materials, spec, color);

    spawn_sphere(
        commands,
        meshes,
        materials,
        spec.impact() + Vec3::Y * 0.4,
        0.6,
        color,
        0.75,
        3.5,
        Vec3::splat(0.2),
        VfxExpandFade::new(0.35, 0.2, 1.6),
    );
}
