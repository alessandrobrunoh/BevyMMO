//! Ground Slam – Hammer secondary frontal AoE.
//!
//! Visual: **wide cone** pointing forward from caster (shockwave front) +
//! expanding ground disc + vertical rising plane.

use bevy::prelude::*;

use crate::spells::ability_vfx::lifecycle::VfxExpandFade;
use crate::spells::ability_vfx::{palette, spawn_matching_footprint, AbilityVfxSpec};

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: &AbilityVfxSpec,
) {
    let color = palette::HAMMER;
    let base = spec.impact();
    let dir = spec.direction();
    spawn_matching_footprint(commands, meshes, materials, spec, color);

    // Vertical rising crack plane
    let plane_mesh = meshes.add(Cuboid::from_size(Vec3::new(
        0.08,
        1.2,
        spec.radius.max(1.0),
    )));
    let plane_mat = super::vfx_material(materials, Color::srgb(1.0, 0.95, 0.7), 0.5, 3.0);
    let mut plane_tfm = Transform::from_translation(base + Vec3::Y * 0.6);
    plane_tfm.look_at(base + dir, Vec3::Y);
    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(plane_mat),
        plane_tfm,
        crate::spells::effects::SpellVisual,
        VfxExpandFade::new(0.32, 0.1, 1.5),
    ));
}
