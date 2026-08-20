//! Volley – Bow secondary multi-projectile.
//!
//! Visual: **fan of small spheres** spreading in an arc from caster toward
//! impact area + wide ground disc.

use bevy::prelude::*;

use crate::spells::ability_vfx::lifecycle::VfxExpandFade;
use crate::spells::ability_vfx::{palette, spawn_matching_footprint, spawn_sphere, AbilityVfxSpec};

const ARROW_COUNT: usize = 5;
const CAST_HEIGHT: f32 = 1.15;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: &AbilityVfxSpec,
) {
    let color = palette::BOW;
    spawn_matching_footprint(commands, meshes, materials, spec, color);
    let base_dir = spec.direction();
    let spread = spec.cone_angle_deg.unwrap_or(70.0);

    // Fan of arrows (small spheres representing projectiles)
    for i in 0..ARROW_COUNT {
        let frac = i as f32 / (ARROW_COUNT.saturating_sub(1)) as f32; // 0..1
        let angle_offset = (frac - 0.5) * spread.to_radians();
        // Rotate base_dir around Y
        let rot = Quat::from_rotation_y(angle_offset);
        let dir = rot * base_dir;
        let offset = dir * 1.2;

        spawn_sphere(
            commands,
            meshes,
            materials,
            spec.start + Vec3::Y * CAST_HEIGHT + offset,
            0.1,
            color,
            0.85,
            3.5,
            Vec3::splat(0.12 + frac * 0.08),
            VfxExpandFade::new(0.22, 0.08, 0.5),
        );
    }

}
