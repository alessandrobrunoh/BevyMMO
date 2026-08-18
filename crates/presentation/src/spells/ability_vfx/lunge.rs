//! Lunge – Sword secondary dash-strike.
//!
//! Visual: **elongated capsule** thrusting forward from caster (the lunge
//! trail) + tip impact burst.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_capsule, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime};

const LUNGE_LENGTH: f32 = 2.8;
const LUNGE_RADIUS: f32 = 0.1;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::SWORD;
    let dir = (effect.end - effect.start).normalize_or_zero();
    let base = effect.start + Vec3::Y * 0.85;

    // Lunge trail – capsule along strike direction
    spawn_capsule(
        commands,
        meshes,
        materials,
        base + dir * (LUNGE_LENGTH * 0.5),
        LUNGE_RADIUS,
        LUNGE_LENGTH,
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
        effect.end + Vec3::Y * 0.5,
        0.3,
        Color::WHITE.with_hue(0.12), // bright gold-white
        0.9,
        5.0,
        Vec3::splat(0.15),
        VfxExpandFade::new(0.16, 0.15, 0.9),
    );
}
