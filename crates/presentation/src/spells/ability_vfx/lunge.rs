//! Lunge – Sword secondary dash-strike.
//!
//! Visual: **elongated capsule** thrusting forward from caster (the lunge
//! trail) + tip impact burst.

use bevy::prelude::*;

use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime};
use crate::spells::ability_vfx::{
    palette, spawn_capsule, spawn_matching_footprint, spawn_sphere, AbilityVfxSpec,
};

const LUNGE_RADIUS: f32 = 0.1;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: &AbilityVfxSpec,
) {
    let color = palette::SWORD;
    spawn_matching_footprint(commands, meshes, materials, spec, color);
    let dir = spec.direction();
    let base = spec.start + Vec3::Y * 0.85;
    let length = spec.radius.max(1.0);

    // Lunge trail – capsule along strike direction
    spawn_capsule(
        commands,
        meshes,
        materials,
        base + dir * (length * 0.5),
        LUNGE_RADIUS,
        length,
        color,
        0.75,
        4.0,
        VfxLifetime::new(0.22),
    );

    // Tip impact
    spawn_sphere(
        commands,
        meshes,
        materials,
        spec.end + Vec3::Y * 0.5,
        0.3,
        Color::WHITE.with_hue(0.12), // bright gold-white
        0.9,
        5.0,
        Vec3::splat(0.15),
        VfxExpandFade::new(0.16, 0.15, 0.9),
    );
}
