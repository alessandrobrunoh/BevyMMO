//! Ground markers for replicated `aoe_region` rows.

use bevy::prelude::*;
use bevymmo_gameplay::abilities::{AbilityGeometry, AbilityId, BaseAbilityRegistry};
use bevymmo_network::world_components::{AoeZone, Position};

use crate::spells::ability_vfx::{ground_sector_mesh, ground_yaw_towards, vfx_glow};
use crate::spells::effects::SpellVisual;

pub fn spawn_aoe_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    abilities: Res<BaseAbilityRegistry>,
    added: Query<(Entity, &Position, &AoeZone), (Added<AoeZone>, Without<Mesh3d>)>,
) {
    for (entity, position, zone) in &added {
        let warning = zone.pending_delay_seconds > 0.0;
        // Same aqua as the aim preview so the hitbox does not look like a
        // second, differently-coloured ability.
        let color = if warning {
            Color::srgb(0.2, 0.95, 0.95)
        } else {
            Color::srgb(0.45, 0.85, 1.0)
        };
        let cone = cone_draw(zone, &abilities);
        let (mesh, transform) = if let Some((angle, direction)) = cone {
            let mesh = meshes.add(ground_sector_mesh(zone.radius.max(0.1), angle));
            let transform = Transform::from_translation(position.0 + Vec3::Y * 0.04)
                .with_rotation(ground_yaw_towards(direction));
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
                base_color: color.with_alpha(0.4),
                emissive: vfx_glow(color, 1.5),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
            transform,
            SpellVisual,
        ));
    }
}

fn cone_draw(zone: &AoeZone, abilities: &BaseAbilityRegistry) -> Option<(f32, Vec3)> {
    if let Some(angle) = zone.cone_angle_deg.filter(|angle| *angle > 1.0) {
        return Some((angle, zone.direction));
    }
    let ability = abilities.get(&AbilityId::new(zone.spell_id.clone()))?;
    match ability.geometry() {
        AbilityGeometry::Cone { angle_deg, .. } => Some((angle_deg, zone.direction)),
        _ => None,
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
        transform.scale = Vec3::new(pulse, 1.0, pulse);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleave_draws_as_a_cone_even_if_the_row_omits_the_angle() {
        let abilities = bevymmo_content::ability_definitions::default_base_abilities();
        let zone = AoeZone {
            radius: 5.0,
            remaining_seconds: 0.15,
            pending_delay_seconds: 0.0,
            spell_id: "cleave".into(),
            cone_angle_deg: None,
            direction: Vec3::Z,
        };
        let (angle, dir) = cone_draw(&zone, &abilities).expect("cone");
        assert!((angle - 85.0).abs() < f32::EPSILON);
        assert_eq!(dir, Vec3::Z);
    }
}
