//! Rush – Gauntlets secondary dash.
//!
//! Visual: **stretched capsule** along dash path (motion streak) + speed-line
//! cones at start and end points.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_capsule, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime};

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::GAUNTLETS;
    let dir = (effect.end - effect.start).normalize_or_zero();
    let dist = (effect.end - effect.start).length();
    let mid = (effect.start + effect.end) * 0.5 + Vec3::Y * 0.85;

    // Motion streak – elongated capsule along path
    let streak_len = dist.max(1.0);
    spawn_capsule(
        commands,
        meshes,
        materials,
        mid,
        0.12,
        streak_len,
        color.with_alpha(0.5),
        0.4,
        2.5,
        VfxLifetime::new(0.2),
    );

    // Start burst (launch)
    spawn_sphere(
        commands,
        meshes,
        materials,
        effect.start + Vec3::Y * 0.85,
        0.22,
        color,
        0.85,
        4.0,
        Vec3::splat(0.18),
        VfxExpandFade::new(0.12, 0.15, 0.8),
    );

    // End cone (arrival shock)
    let mesh = meshes.add(Cone { radius: 0.35, height: 0.6 });
    let mat = super::vfx_material(materials, color, 0.6, 3.0);
    let mut tfm = Transform::from_translation(effect.end + Vec3::Y * 0.85);
    tfm.look_at(effect.end + dir, Vec3::Y);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        tfm,
        crate::spells::effects::SpellVisual,
        VfxLifetime::new(0.16),
    ));
}
