//! Bundle e helper di spawn per le entità di gioco.

use bevy::color::Color;
use bevy::ecs::entity::Entity;
use bevy::prelude::*;
use lightyear::prelude::{NetworkTarget, Replicate};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::network::protocol::{EntityColor, LookDirection, NetworkEntityId, Position};

use super::components::{EntityKind, EntityState, GameEntity};
use super::definition::EntityDefinition;
use crate::stats::components::{CombatStats, MovementStats, VitalStats};

static NEXT_NETWORK_ENTITY_ID: AtomicU64 = AtomicU64::new(1);

fn next_network_entity_id() -> NetworkEntityId {
    NetworkEntityId(NEXT_NETWORK_ENTITY_ID.fetch_add(1, Ordering::Relaxed))
}

/// Stato comune di un'entità di gameplay replicata.
///
/// I tipi concreti aggiungono il proprio bundle e, se necessario, componenti
/// Lightyear speciali come ownership, prediction e interpolation.
#[derive(Bundle)]
pub struct GameEntityBundle {
    game_entity: GameEntity,
    state: EntityState,
    movement_stats: MovementStats,
    combat_stats: CombatStats,
    vital_stats: VitalStats,
    position: Position,
    look_direction: LookDirection,
    color: EntityColor,
    network_entity_id: NetworkEntityId,
    entity_kind: EntityKind,
    replicate: Replicate,
}

impl GameEntityBundle {
    pub fn new(
        position: Position,
        color: EntityColor,
        stats_data: crate::stats::components::StatsBundleData,
        entity_kind: EntityKind,
        replication_target: NetworkTarget,
    ) -> Self {
        let (movement, combat, vital) = stats_data.into_components();
        Self {
            game_entity: GameEntity,
            state: EntityState::default(),
            movement_stats: movement,
            combat_stats: combat,
            vital_stats: vital,
            position,
            look_direction: LookDirection::default(),
            color,
            network_entity_id: next_network_entity_id(),
            entity_kind,
            replicate: Replicate::to_clients(replication_target),
        }
    }
}

/// Spawna un'entità standard completa di stato comune, bundle specifico e
/// replica. Per entità con requisiti di ownership speciali usa
/// [`GameEntityBundle`] e aggiunge le componenti Lightyear necessarie.
pub fn spawn_entity<T: EntityDefinition>(commands: &mut Commands) -> Entity {
    let entity = commands
        .spawn((
            GameEntityBundle::new(
                Position(T::initial_position()),
                EntityColor(T::initial_color()),
                T::stats(),
                T::entity_kind(),
                T::replication_target(),
            ),
            T::bundle(),
        ))
        .id();
    info!("Spawned {} entity {:?}", T::name(), entity);
    entity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_spawn_contains_shared_gameplay_components() {
        let mut world = World::new();
        let stats = crate::stats::components::StatsBundleData {
            movement: crate::stats::components::MovementStats { speed: 0.15 },
            combat: crate::stats::components::CombatStats {
                attack_power: 10.0,
                armor: 0.0,
            },
            vital: crate::stats::components::VitalStats {
                current_health: 25.0,
                max_health: 25.0,
                max_mana: 50.0,
                mana_regeneration: 2.0,
            },
        };
        let entity = world
            .spawn(GameEntityBundle::new(
                Position(Vec3::ZERO),
                EntityColor(Color::WHITE),
                stats,
                EntityKind::Neutral,
                NetworkTarget::All,
            ))
            .id();

        let entity_ref = world.entity(entity);
        assert!(entity_ref.contains::<GameEntity>());
        assert!(entity_ref.contains::<EntityState>());
        assert!(entity_ref.contains::<MovementStats>());
        assert!(entity_ref.contains::<CombatStats>());
        assert!(entity_ref.contains::<VitalStats>());
        assert!(entity_ref.contains::<Position>());
        assert!(entity_ref.contains::<LookDirection>());
        assert!(entity_ref.contains::<EntityColor>());
        assert!(entity_ref.contains::<NetworkEntityId>());
        assert!(entity_ref.contains::<EntityKind>());
        assert!(entity_ref.contains::<Replicate>());
    }

    #[test]
    fn game_entity_bundle_contains_entity_kind() {
        let mut world = World::new();
        let stats = crate::stats::components::StatsBundleData {
            movement: crate::stats::components::MovementStats { speed: 0.0 },
            combat: crate::stats::components::CombatStats {
                attack_power: 0.0,
                armor: 0.0,
            },
            vital: crate::stats::components::VitalStats {
                current_health: 1_000_000_000.0,
                max_health: 1_000_000_000.0,
                max_mana: 0.0,
                mana_regeneration: 0.0,
            },
        };
        let entity = world
            .spawn(GameEntityBundle::new(
                Position(Vec3::new(8.0, 0.0, 0.0)),
                EntityColor(Color::srgb(0.7, 0.1, 0.1)),
                stats,
                crate::plugins::entity::components::EntityKind::Hostile,
                NetworkTarget::All,
            ))
            .id();

        let entity_ref = world.entity(entity);
        assert!(entity_ref.contains::<EntityKind>());
    }
}
