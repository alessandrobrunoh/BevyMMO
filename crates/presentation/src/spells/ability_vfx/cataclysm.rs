//! Cataclysm – Hammer ultimate.
//!
//! Visual: **massive layered explosion** – large outer sphere, mid sphere,
//! inner core, plus multiple ground rings at different radii. The "nuke" of
//! hammer abilities.

use bevy::prelude::*;

use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxSpinExpand};
use crate::spells::ability_vfx::{
    palette, spawn_disc, spawn_matching_footprint, spawn_sphere, spawn_torus, AbilityVfxSpec,
};

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: &AbilityVfxSpec,
) {
    let color = palette::HAMMER;
    let center = spec.impact();
    spawn_matching_footprint(commands, meshes, materials, spec, color);

    // Outer shockwave sphere (largest, slowest)
    spawn_sphere(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 1.0,
        1.8,
        color.with_alpha(0.3),
        0.3,
        2.0,
        Vec3::splat(0.1),
        VfxExpandFade::new(0.65, 0.1, 3.5),
    );

    // Mid combustion sphere
    spawn_sphere(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.8,
        1.0,
        Color::srgb(1.0, 0.6, 0.1), // orange-yellow core
        0.7,
        4.0,
        Vec3::splat(0.15),
        VfxExpandFade::new(0.5, 0.15, 2.2),
    );

    // Inner white-hot core
    spawn_sphere(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.6,
        0.5,
        Color::srgb(1.0, 1.0, 0.9),
        0.9,
        6.0,
        Vec3::splat(0.2),
        VfxExpandFade::new(0.35, 0.2, 1.4),
    );

    // Ground rings at fractions of the preview radius
    for (i, &(frac, hue_offset)) in [(1.0, 0.0), (0.72, 0.04), (0.48, 0.08)].iter().enumerate() {
        spawn_disc(
            commands,
            meshes,
            materials,
            center,
            spec.radius * frac,
            0.06,
            color.with_hue(hue_offset),
            0.35 - i as f32 * 0.08,
            2.5 - i as f32 * 0.5,
            Vec3::splat(0.05 + i as f32 * 0.05),
            VfxSpinExpand::new(
                0.55 + i as f32 * 0.08,
                0.05,
                1.6 + i as f32 * 0.3,
                4.0 - i as f32,
            ),
        );
    }

    // Outer rotating shockwave ring (torus)
    spawn_torus(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.15,
        spec.radius.max(0.5),
        0.1,
        Color::srgb(1.0, 0.5, 0.05),
        0.25,
        3.5,
        VfxSpinExpand::new(0.7, 0.2, 2.8, 5.0),
    );
}
