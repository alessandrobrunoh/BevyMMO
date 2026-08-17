//! Sistemi per il target indicator (anello rosso sotto il target).

use bevy::prelude::*;
use bevymmo_client::targeting::CurrentTarget;
use bevymmo_gameplay::entity::components::EntityKind;
use bevymmo_network::network::protocol::Position;

use super::components::{TargetRingTarget, TargetSelectionRing};

/// Crea un nuovo anello di selezione per il target specificato.
pub fn spawn_target_ring(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    target_entity: Entity,
    target_position: Vec3,
) -> Entity {
    // Mesh: Torus
    let ring_mesh = meshes.add(Torus {
        major_radius: 1.0,
        minor_radius: 0.05,
    });

    // Materiale: rosso emissivo
    let ring_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.1, 0.1),
        emissive: Color::srgb(0.8, 0.0, 0.0).into(),
        ..default()
    });

    // Posizione: leggermente sopra il terreno
    let ring_position = target_position + Vec3::Y * 0.04;

    commands
        .spawn((
            Mesh3d(ring_mesh),
            MeshMaterial3d(ring_material),
            Transform::from_translation(ring_position)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::PI / 2.0)),
            TargetSelectionRing,
            TargetRingTarget {
                entity: target_entity,
            },
        ))
        .id()
}

/// Sistema: aggiorna la posizione dell'anello in base al target corrente.
pub fn update_target_ring(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_target: Res<CurrentTarget>,
    target_query: Query<(&Position, Option<&EntityKind>)>,
    mut ring_query: Query<(Entity, &TargetRingTarget, &mut Transform)>,
) {
    let current_target_entity = current_target.entity;

    // Check if we have a target
    let target_entity = match current_target_entity {
        Some(entity) => entity,
        None => {
            // No target: remove any existing ring
            for (ring_entity, ..) in ring_query.iter() {
                commands.entity(ring_entity).despawn();
            }
            return;
        }
    };

    // Try to get the target's position
    let (target_position, _entity_kind) = match target_query.get(target_entity) {
        Ok((pos, kind)) => (pos, kind),
        Err(_) => {
            // Target doesn't exist or doesn't have Position: remove any existing ring
            for (ring_entity, ..) in ring_query.iter() {
                commands.entity(ring_entity).despawn();
            }
            return;
        }
    };

    // Check if we already have a ring for this target
    let mut existing_ring_for_target = None;
    for (ring_entity, ring_target, _) in ring_query.iter() {
        if ring_target.entity == target_entity {
            existing_ring_for_target = Some(ring_entity);
            break;
        }
    }

    match existing_ring_for_target {
        Some(ring_entity) => {
            // Ring exists: update its position
            if let Ok((_, _, mut transform)) = ring_query.get_mut(ring_entity) {
                let new_position = target_position.0 + Vec3::Y * 0.04;
                transform.translation = new_position;
            }
        }
        None => {
            // No ring for this target: remove any old rings and create a new one
            for (ring_entity, ..) in ring_query.iter() {
                commands.entity(ring_entity).despawn();
            }
            spawn_target_ring(
                &mut commands,
                &mut meshes,
                &mut materials,
                target_entity,
                target_position.0,
            );
        }
    }
}

/// Sistema: rimuove gli anelli quando il target non è più valido.
pub fn cleanup_target_rings(
    mut commands: Commands,
    target_query: Query<&Position>,
    ring_query: Query<(Entity, &TargetRingTarget)>,
) {
    for (ring_entity, ring_target) in ring_query.iter() {
        if target_query.get(ring_target.entity).is_err() {
            commands.entity(ring_entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indicator_app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.add_systems(Update, update_target_ring);
        app
    }

    #[test]
    fn ring_is_removed_when_target_is_cleared() {
        let mut app = indicator_app();

        // Setup: target con position
        let target = app
            .world_mut()
            .spawn((Position(Vec3::new(10.0, 0.0, 5.0)), EntityKind::Hostile))
            .id();

        // Setup: imposta il target corrente
        app.world_mut().insert_resource(CurrentTarget::new(target));

        // Setup: spawn un anello
        let ring = app
            .world_mut()
            .spawn((
                TargetSelectionRing,
                TargetRingTarget { entity: target },
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();

        app.update();

        // Verifica: l'anello dovrebbe esistere
        assert!(app.world().entities().contains(ring));

        // Clear target
        app.world_mut().resource_mut::<CurrentTarget>().clear();

        app.update();

        // Verifica: l'anello dovrebbe essere stato rimosso
        assert!(!app.world().entities().contains(ring));
    }

    #[test]
    fn ring_follows_target_position() {
        let mut app = indicator_app();

        // Setup: target con position
        let target = app
            .world_mut()
            .spawn(Position(Vec3::new(0.0, 0.0, 0.0)))
            .id();

        // Setup: imposta il target corrente
        app.world_mut().insert_resource(CurrentTarget::new(target));

        // Setup: spawn un anello
        app.world_mut().spawn((
            TargetSelectionRing,
            TargetRingTarget { entity: target },
            Transform::from_translation(Vec3::ZERO),
        ));

        app.update();

        // Muovi il target
        let new_pos = Vec3::new(10.0, 0.0, 15.0);
        app.world_mut()
            .entity_mut(target)
            .get_mut::<Position>()
            .unwrap()
            .0 = new_pos;

        app.update();

        // Verifica: l'anello dovrebbe essere alla nuova posizione (con offset Y)
        let mut ring_query = app
            .world_mut()
            .query_filtered::<&Transform, With<TargetSelectionRing>>();
        let ring_transform = ring_query.single(app.world()).unwrap();
        let expected = new_pos + Vec3::Y * 0.04;
        assert_eq!(ring_transform.translation, expected);
    }

    #[test]
    fn ring_is_replaced_when_target_changes() {
        let mut app = indicator_app();

        // Setup: due target
        let target1 = app
            .world_mut()
            .spawn(Position(Vec3::new(0.0, 0.0, 0.0)))
            .id();
        let target2 = app
            .world_mut()
            .spawn(Position(Vec3::new(10.0, 0.0, 5.0)))
            .id();

        // Setup: imposta il primo target
        app.world_mut().insert_resource(CurrentTarget::new(target1));

        // Setup: spawn un anello per il primo target
        let ring1 = app
            .world_mut()
            .spawn((
                TargetSelectionRing,
                TargetRingTarget { entity: target1 },
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();

        app.update();

        // Cambia al secondo target
        app.world_mut().resource_mut::<CurrentTarget>().set(target2);

        app.update();

        // Verifica: il primo anello dovrebbe essere stato rimosso
        assert!(!app.world().entities().contains(ring1));

        // Verifica: dovrebbe esserci un nuovo anello
        let ring_count = app
            .world_mut()
            .query::<&TargetSelectionRing>()
            .iter(app.world())
            .count();
        assert_eq!(ring_count, 1);
    }
}
