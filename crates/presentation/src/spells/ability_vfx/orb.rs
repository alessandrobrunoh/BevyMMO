//! Orb – Focus primary projectile.
//!
//! Visual: **nested spheres** (outer shell + inner core) that travel together,
//! giving a layered energy-orb appearance.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxLifetime};

const CAST_HEIGHT: f32 = 1.1;
const OUTER_RADIUS: f32 = 0.35;
const INNER_RADIUS: f32 = 0.18;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::FOCUS;
    let pos = effect.start + Vec3::Y * CAST_HEIGHT;

    // Outer translucent shell
    spawn_sphere(
        commands,
        meshes,
        materials,
        pos,
        OUTER_RADIUS,
        color,
        0.35,
        2.5,
        Vec3::splat(0.8),
        VfxExpandFade::new(0.4, 0.6, 1.4),
    );

    // Inner bright core
    spawn_sphere(
        commands,
        meshes,
        materials,
        pos,
        INNER_RADIUS,
        Color::srgb(0.7, 0.85, 1.0), // lighter azure
        0.9,
        5.0,
        Vec3::splat(0.5),
        VfxExpandFade::new(0.35, 0.3, 1.0),
    );

    // Small trail sphere behind (motion blur hint)
    spawn_sphere(
        commands,
        meshes,
        materials,
        pos - (effect.end - effect.start).normalize_or_zero() * 0.3,
        0.12,
        color,
        0.25,
        2.0,
        Vec3::splat(0.4),
        VfxLifetime::new(0.15),
    );
}
