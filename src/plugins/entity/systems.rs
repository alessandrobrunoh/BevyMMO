//! Sistemi condivisi da tutte le entità di gioco.
//!
//! `mark_dead_entities` gestisce la transizione allo stato `Dead` quando una
//! `VitalStats` scende a zero. Non despawn: le entità morte restano in scena
//! (`EntityState::Dead`) finché un sistema di respawn esplicito non le
//! riporta in vita, permettendo UI di morte, animazioni e visual feedback.
use bevy::prelude::*;

use super::components::{EntityKind, EntityState};
use super::events::DeathEvent;
use crate::stats::components::VitalStats;

/// Transizione `Idle/Moving -> Dead` quando `VitalStats.is_dead()`.
///
/// Server-authoritative: gira solo sul server, lo stato viene poi replicato
/// ai client tramite `EntityState`.
pub fn mark_dead_entities(
    mut death_events: MessageWriter<DeathEvent>,
    mut query: Query<(Entity, &mut EntityState, &VitalStats, &EntityKind), Changed<VitalStats>>,
) {
    for (entity, mut state, vital, kind) in query.iter_mut() {
        if vital.is_dead() && *state != EntityState::Dead {
            *state = EntityState::Dead;
            death_events.write(DeathEvent {
                entity,
                kind: *kind,
            });
            info!("Entity {:?} ({:?}) died", entity, kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::entity::components::SpawnPoint;
    use crate::stats::components::{CombatStats, MovementStats};

    #[test]
    fn dead_entity_transitions_to_dead_state() {
        let mut app = App::new();
        app.add_message::<DeathEvent>();
        app.add_systems(Update, mark_dead_entities);

        let entity = app
            .world_mut()
            .spawn((
                EntityState::Idle,
                VitalStats {
                    current_health: 0.0,
                    max_health: 100.0,
                    max_mana: 50.0,
                    mana_regeneration: 1.0,
                },
                EntityKind::Hostile,
                SpawnPoint(Vec3::ZERO),
            ))
            .id();

        app.update();

        let state = app
            .world()
            .entity(entity)
            .get::<EntityState>()
            .copied()
            .expect("state");
        assert_eq!(state, EntityState::Dead);
    }

    #[test]
    fn alive_entities_are_not_marked_dead() {
        let mut app = App::new();
        app.add_message::<DeathEvent>();
        app.add_systems(Update, mark_dead_entities);

        let entity = app
            .world_mut()
            .spawn((
                EntityState::Idle,
                VitalStats {
                    current_health: 50.0,
                    max_health: 100.0,
                    max_mana: 50.0,
                    mana_regeneration: 1.0,
                },
                EntityKind::Player,
                SpawnPoint(Vec3::ZERO),
            ))
            .id();
        app.update();

        let state = app
            .world()
            .entity(entity)
            .get::<EntityState>()
            .copied()
            .expect("state");
        assert_eq!(state, EntityState::Idle);
    }

    #[test]
    fn already_dead_entities_are_not_re_processed() {
        let mut app = App::new();
        app.add_message::<DeathEvent>();
        app.add_systems(Update, mark_dead_entities);

        let entity = app
            .world_mut()
            .spawn((
                EntityState::Dead,
                VitalStats {
                    current_health: 0.0,
                    max_health: 100.0,
                    max_mana: 50.0,
                    mana_regeneration: 1.0,
                },
                EntityKind::Hostile,
                // Altri componenti richiesti per matchare il resto del bundle.
                MovementStats { speed: 0.0 },
                CombatStats {
                    attack_power: 0.0,
                    armor: 0.0,
                },
            ))
            .id();
        // Modifichiamo `VitalStats` per forzare `Changed<VitalStats>` senza
        // cambiare il valore effettivo: ci assicura che il sistema giri.
        app.update();
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<VitalStats>()
            .unwrap()
            .current_health = 0.0;
        app.update();

        // Lo stato resta `Dead`: non c'è transizione spuria.
        let state = app
            .world()
            .entity(entity)
            .get::<EntityState>()
            .copied()
            .expect("state");
        assert_eq!(state, EntityState::Dead);
    }
}
