//! Power Shot – Bow primary charged projectile.
//!
//! Visual: **elongated sharp box** (arrow shape) streaking from caster to
//! target + trailing cone (speed lines).

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime};

const CAST_HEIGHT: f32 = 1.15;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::BOW;
    let dir = (effect.end - effect.start).normalize_or_zero();

    // Nock / release flash
    spawn_sphere(
        commands,
        meshes,
        materials,
        effect.start + Vec3::Y * CAST_HEIGHT,
        0.15,
        color,
        0.9,
        4.0,
        Vec3::splat(0.12),
        VfxExpandFade::new(0.14, 0.1, 0.6),
    );

    // Arrow shaft – slim box
    let mesh = meshes.add(Cuboid::from_size(Vec3::new(0.04, 0.04, 1.4)));
    let mat = super::vfx_material(materials, color, 0.9, 4.0);
    let mut tfm = Transform::from_translation(
        effect.start + Vec3::Y * CAST_HEIGHT + dir * 0.7,
    );
    tfm.look_at(effect.end, Vec3::Y);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        tfm,
        crate::spells::effects::SpellVisual,
        VfxLifetime::new(0.2),
    ));

    // Speed-trail cone behind arrow
    let trail_mesh = meshes.add(Cone { radius: 0.15, height: 0.8 });
    let trail_mat = super::vfx_material(materials, color, 0.4, 2.0);
    let mut trail_tfm =
        Transform::from_translation(effect.start + Vec3::Y * CAST_HEIGHT - dir * 0.3);
    trail_tfm.look_at(effect.start, Vec3::Y);
    commands.spawn((
        Mesh3d(trail_mesh),
        MeshMaterial3d(trail_mat),
        trail_tfm,
        crate::spells::effects::SpellVisual,
        VfxLifetime::new(0.18),
    ));
}
