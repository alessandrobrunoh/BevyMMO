//! Systems shared by all game entities.
//!
//! `mark_dead_entities` manages the transition to `Dead` state when a
//! `VitalStats` drops to zero. No despawning: dead entities remain in the scene
//! (`EntityState::Dead`) until an explicit respawn system brings them back to life,
//! allowing death UI, animations, and visual feedback.
use bevy::prelude::*;

use super::components::{EntityKind, EntityState};
use super::events::DeathEvent;
use crate::stats::components::VitalStats;

/// Transition `Idle/Moving -> Dead` when `VitalStats.is_dead()`.
///
/// Server-authoritative: runs only on the server, state is then replicated
/// to clients via `EntityState`.
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
                // Other components required to match the rest of the bundle.
                MovementStats { speed: 0.0 },
                CombatStats {
                    attack_power: 0.0,
                    armor: 0.0,
                },
            ))
            .id();
        // Modify `VitalStats` to trigger `Changed<VitalStats>` without
        // changing the actual value: ensures the system runs.
        app.update();
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<VitalStats>()
            .unwrap()
            .current_health = 0.0;
        app.update();

        // State remains `Dead`: no spurious transition.
        let state = app
            .world()
            .entity(entity)
            .get::<EntityState>()
            .copied()
            .expect("state");
        assert_eq!(state, EntityState::Dead);
    }
}

