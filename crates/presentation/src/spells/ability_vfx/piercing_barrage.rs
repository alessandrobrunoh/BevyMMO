//! Piercing Barrage – Bow ultimate.
//!
//! Visual: **line of elongated boxes** (arrows) in a tight row, all oriented
//! same direction + ground scar (long thin box).

use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::ability_vfx::{palette, spawn_box, spawn_sphere};
use crate::spells::ability_vfx::lifecycle::VfxLifetime;

const BARRAGE_COUNT: usize = 7;
const ARROW_LEN: f32 = 1.6;
const ARROW_SPACING: f32 = 0.35;

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = palette::BOW;
    let dir = (effect.end - effect.start).normalize_or_zero();
    let right = dir.cross(Vec3::Y).normalize_or_zero();
    let center = effect.end;

    // Muzzle burst
    spawn_sphere(
        commands,
        meshes,
        materials,
        effect.start + Vec3::Y * 1.15,
        0.25,
        color,
        0.9,
        4.5,
        Vec3::splat(0.2),
        crate::spells::ability_vfx::lifecycle::VfxExpandFade::new(0.2, 0.2, 1.0),
    );

    // Line of arrows
    for i in 0..BARRAGE_COUNT {
        let lateral = (i as f32 - (BARRAGE_COUNT as f32 - 1.0) / 2.0) * ARROW_SPACING;
        let pos = center + right * lateral + Vec3::Y * 0.6;

        let mesh = meshes.add(Cuboid::from_size(Vec3::new(0.03, 0.03, ARROW_LEN)));
        let mat = super::vfx_material(materials, color, 0.85, 4.0);
        let mut tfm = Transform::from_translation(pos);
        tfm.look_at(pos + dir, Vec3::Y);
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            tfm,
            crate::spells::effects::SpellVisual,
            VfxLifetime::new(0.3 + i as f32 * 0.03),
        ));
    }

    // Ground scar – long box
    spawn_box(
        commands,
        meshes,
        materials,
        center + Vec3::Y * 0.02,
        Vec3::new(0.08, 0.06, BARRAGE_COUNT as f32 * ARROW_SPACING + ARROW_LEN),
        color.with_hue(-0.05),
        0.35,
        2.0,
        VfxLifetime::new(0.5),
    );
}
