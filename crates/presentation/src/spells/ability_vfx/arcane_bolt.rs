//! Arcane Bolt – Staff primary projectile.
//!
//! Visual: a slender **elongated capsule** (bolt shape) that shoots from
//! caster toward impact point + a small muzzle-burst sphere at cast position.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime};

const CAST_HEIGHT: f32 = 1.1;
const BOLT_LENGTH: f32 = 1.8;
const BOLT_RADIUS: f32 = 0.08;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::STAFF;
    let dir = (effect.end - effect.start).normalize_or_zero();

    // Muzzle flash at caster hand
    spawn_sphere(
        commands,
        meshes,
        materials,
        effect.start + Vec3::Y * CAST_HEIGHT,
        0.2,
        color,
        0.9,
        4.0,
        Vec3::splat(0.15),
        VfxExpandFade::new(0.18, 0.15, 0.7),
    );

    // The bolt itself – capsule oriented along direction
    let mesh = meshes.add(Capsule3d::new(BOLT_RADIUS, BOLT_LENGTH));
    let mat = super::vfx_material(materials, color, 0.85, 5.0);

    let mut transform = Transform::from_translation(
        effect.start + Vec3::Y * CAST_HEIGHT + dir * BOLT_LENGTH * 0.5,
    );
    transform.look_at(effect.end, Vec3::Y);

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        transform,
        crate::spells::effects::SpellVisual,
        VfxLifetime::new(0.25),
    ));
}
