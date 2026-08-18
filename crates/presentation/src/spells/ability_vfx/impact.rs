//! Impact – Gauntlets ultimate.
//!
//! Visual: **large expanding sphere** (ground pound) + **outward-radiating
//! box spikes** (seismic cracks) + **rising dust cylinder**.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_box, spawn_capsule, spawn_disc, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::{VfxExpandFade, VfxFall, VfxPulseRing};

const SPIKE_COUNT: usize = 8;
const SPIKE_LEN: f32 = 1.8;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::GAUNTLETS;
    let center = effect.end;

    // Main impact sphere (expanding shockwave)
    spawn_sphere(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.6,
        1.2,
        color,
        0.6,
        3.5,
        Vec3::splat(0.1),
        VfxExpandFade::new(0.45, 0.1, 2.8),
    );

    // White-hot core
    spawn_sphere(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.4,
        0.45,
        Color::srgb(1.0, 0.9, 0.75),
        0.88,
        6.0,
        Vec3::splat(0.18),
        VfxExpandFade::new(0.3, 0.15, 1.5),
    );

    // Radiating crack spikes (boxes on ground plane)
    for i in 0..SPIKE_COUNT {
        let angle = (i as f32 / SPIKE_COUNT as f32) * std::f32::consts::TAU;
        let offset = Vec3::new(angle.cos(), 0.0, angle.sin());
        let spike_center = center + offset * (SPIKE_LEN * 0.5) + Vec3::Y * 0.03;

        spawn_box(
            commands,
            meshes,
            materials,
            spike_center,
            Vec3::new(0.06, 0.05, SPIKE_LEN),
            color.with_hue(i as f32 * 0.02),
            0.45,
            2.5,
            VfxExpandFade::new(0.38 + i as f32 * 0.02, 0.03, 0.8),
        );
    }

    // Expanding ground disc
    spawn_disc(
        commands,
        meshes,
        materials,
        center,
        2.2,
        0.05,
        color.with_hue(0.04),
        0.35,
        2.5,
        Vec3::splat(0.08),
        VfxPulseRing::new(0.06, 0.35),
    );

    // Rising dust column
    spawn_capsule(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.8,
        0.3,
        1.4,
        Color::srgb(0.95, 0.88, 0.72), // warm dust
        0.35,
        1.5,
        VfxFall::new(0.4, 0.1, 1.8),
    );
}
