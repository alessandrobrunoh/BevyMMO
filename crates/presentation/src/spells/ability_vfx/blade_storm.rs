//! Blade Storm – Sword ultimate.
//!
//! Visual: **multiple tilted boxes** (blades) orbiting the caster in a
//! horizontal ring, each spinning on its own axis.

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::lifecycle::VfxSpinExpand;
use crate::spells::ability_vfx::palette;

const BLADE_COUNT: usize = 6;
const ORBIT_RADIUS: f32 = 1.6;
const BLADE_SIZE: Vec3 = Vec3::new(0.08, 0.7, 0.25);

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::SWORD;
    let center = effect.start + Vec3::Y * 1.0;

    for i in 0..BLADE_COUNT {
        let angle = (i as f32 / BLADE_COUNT as f32) * std::f32::consts::TAU;
        let offset = Vec3::new(angle.cos() * ORBIT_RADIUS, 0.0, angle.sin() * ORBIT_RADIUS);
        let pos = center + offset;

        // Tilt each blade outward slightly
        let tilt = Quat::from_rotation_z(0.35) * Quat::from_rotation_y(angle);

        let mesh = meshes.add(Cuboid::from_size(BLADE_SIZE));
        let mat = super::vfx_material(materials, color, 0.8, 4.0);
        let mut tfm = Transform::from_translation(pos);
        tfm.rotation = tilt;
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            tfm,
            crate::spells::effects::SpellVisual,
            VfxSpinExpand::new(0.7, 0.8, 1.3, 18.0 + i as f32 * 2.0),
        ));
    }

    // Inner core sphere
    let core_mesh = meshes.add(Sphere::new(0.35));
    let core_mat = super::vfx_material(materials, Color::WHITE, 0.5, 3.0);
    commands.spawn((
        Mesh3d(core_mesh),
        MeshMaterial3d(core_mat),
        Transform::from_translation(center),
        crate::spells::effects::SpellVisual,
        VfxSpinExpand::new(0.7, 0.6, 1.1, 8.0),
    ));
}
