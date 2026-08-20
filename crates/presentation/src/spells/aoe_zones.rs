//! Ground markers for replicated `aoe_region` rows.

use bevy::prelude::*;
use bevymmo_network::world_components::{AoeZone, Position};

use crate::spells::effects::SpellVisual;

pub fn spawn_aoe_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    added: Query<(Entity, &Position, &AoeZone), (Added<AoeZone>, Without<Mesh3d>)>,
) {
    for (entity, position, zone) in &added {
        let warning = zone.pending_delay_seconds > 0.0;
        let color = if warning {
            Color::srgb(1.0, 0.45, 0.15)
        } else {
            Color::srgb(0.55, 0.25, 0.95)
        };
        let (mesh, transform) = if let Some(angle) = zone.cone_angle_deg {
            let dir = Vec3::new(zone.direction.x, 0.0, zone.direction.z).normalize_or_zero();
            let mesh = meshes.add(crate::spells::ability_vfx::ground_sector_mesh(
                zone.radius.max(0.1),
                angle,
            ));
            let transform = Transform::from_translation(position.0 + Vec3::Y * 0.04)
                .looking_to(if dir == Vec3::ZERO { Vec3::Z } else { dir }, Vec3::Y);
            (mesh, transform)
        } else {
            (
                meshes.add(Cylinder::new(zone.radius.max(0.1), 0.08)),
                Transform::from_translation(position.0 + Vec3::Y * 0.04),
            )
        };
        commands.entity(entity).insert((
            Mesh3d(mesh),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color.with_alpha(0.35),
                emissive: crate::spells::ability_vfx::vfx_glow(color, 1.5),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            transform,
            SpellVisual,
        ));
    }
}

pub fn pulse_aoe_meshes(time: Res<Time>, mut zones: Query<(&AoeZone, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (zone, mut transform) in &mut zones {
        let pulse = if zone.pending_delay_seconds > 0.05 {
            0.9 + 0.08 * (t * 8.0).sin()
        } else {
            1.0
        };
        if zone.cone_angle_deg.is_some() {
            transform.scale = Vec3::splat(pulse);
        } else {
            transform.scale = Vec3::new(pulse, 1.0, pulse);
        }
    }
}
