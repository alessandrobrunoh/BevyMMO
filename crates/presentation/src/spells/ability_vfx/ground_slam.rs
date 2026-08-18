//! Ground Slam – Hammer secondary frontal AoE.
//!
//! Visual: **wide cone** pointing forward from caster (shockwave front) +
//! expanding ground disc + vertical rising plane.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime, VfxPulseRing};
use crate::spells::ability_vfx::{palette, spawn_disc};

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::HAMMER;
    let base = effect.start;
    let dir = (effect.end - effect.start).normalize_or_zero();

    // Shockwave cone – points forward from caster
    let mesh = meshes.add(Cone {
        radius: 2.2,
        height: 1.0,
    });
    let mat = super::vfx_material(materials, color, 0.45, 2.5);
    let mut tfm = Transform::from_translation(base + Vec3::Y * 0.5);
    tfm.look_at(base + dir + Vec3::Y * 0.5, Vec3::Y);
    tfm.rotate_x(std::f32::consts::FRAC_PI_2); // lay cone on its side
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        tfm,
        crate::spells::effects::SpellVisual,
        VfxLifetime::new(0.35),
    ));

    // Expanding ground disc
    spawn_disc(
        commands,
        meshes,
        materials,
        base,
        2.5,
        0.05,
        color.with_hue(0.03),
        0.4,
        2.5,
        Vec3::splat(0.1),
        VfxPulseRing::new(0.08, 0.28),
    );

    // Vertical rising crack plane
    let plane_mesh = meshes.add(Cuboid::from_size(Vec3::new(0.08, 1.2, 3.0)));
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
