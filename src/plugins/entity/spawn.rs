//! Bundle e helper di spawn per le entità di gioco.

use bevy::ecs::entity::Entity;
use bevy::prelude::*;
use lightyear::prelude::{NetworkTarget, Replicate};

use crate::network::protocol::{EntityColor, Position};

use super::components::{EntityState, GameEntity, Health, Stats};
use super::definition::EntityDefinition;

/// Stato comune di un'entità di gameplay replicata.
///
/// I tipi concreti aggiungono il proprio bundle e, se necessario, componenti
/// Lightyear speciali come ownership, prediction e interpolation.
#[derive(Bundle)]
pub struct GameEntityBundle {
    game_entity: GameEntity,
    state: EntityState,
    health: Health,
    stats: Stats,
    position: Position,
    color: EntityColor,
    replicate: Replicate,
}

impl GameEntityBundle {
    pub fn new(
        position: Position,
        color: EntityColor,
        health: Health,
        stats: Stats,
        replication_target: NetworkTarget,
    ) -> Self {
        Self {
            game_entity: GameEntity,
            state: EntityState::default(),
            health,
            stats,
            position,
            color,
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
                T::health(),
                T::stats(),
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
        let entity = world
            .spawn(GameEntityBundle::new(
                Position(Vec3::ZERO),
                EntityColor(Color::WHITE),
                Health::new(25.0),
                Stats::default(),
                NetworkTarget::All,
            ))
            .id();

        let entity_ref = world.entity(entity);
        assert!(entity_ref.contains::<GameEntity>());
        assert!(entity_ref.contains::<EntityState>());
        assert!(entity_ref.contains::<Health>());
        assert!(entity_ref.contains::<Stats>());
        assert!(entity_ref.contains::<Position>());
        assert!(entity_ref.contains::<EntityColor>());
        assert!(entity_ref.contains::<Replicate>());
    }
}
