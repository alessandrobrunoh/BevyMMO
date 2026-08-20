//! Crushing Blow – Hammer primary overhead strike.
//!
//! Visual: **vertical capsule** coming down from above (hammer head shape) +
//! ground shockwave disc on impact.

use bevy::prelude::*;

use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxFall};
use crate::spells::ability_vfx::{palette, spawn_matching_footprint, spawn_sphere, AbilityVfxSpec};

const HAMMER_HEIGHT: f32 = 6.0;
const HAMMER_RADIUS: f32 = 0.35;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: &AbilityVfxSpec,
) {
    let color = palette::HAMMER;
    let target = spec.impact();
    spawn_matching_footprint(commands, meshes, materials, spec, color);

    // Falling hammer head – capsule that drops from high up
    let mesh = meshes.add(Capsule3d::new(HAMMER_RADIUS, 0.6));
    let mat = super::vfx_material(materials, color, 0.9, 4.5);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(target + Vec3::Y * HAMMER_HEIGHT),
        crate::spells::effects::SpellVisual,
        VfxFall::new(0.35, HAMMER_HEIGHT, 0.5),
    ));

    // Impact dust burst
    spawn_sphere(
        commands,
        meshes,
        materials,
        target + Vec3::Y * 0.25,
        0.55,
        Color::srgb(1.0, 0.95, 0.8), // warm dust
        0.7,
        2.0,
        Vec3::splat(0.1),
        VfxExpandFade::new(0.3, 0.1, 1.4),
    );
}
