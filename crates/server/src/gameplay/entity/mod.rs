//! Server-authoritative entity systems and umbrella plugin.
//!
//! `EntityServerPlugin` registers shared entity types (spawn points, death and
//! respawn messages) and the concrete entity plugins (player, enemy, dummy,
//! boss). All gameplay systems here are server-authoritative and gated by
//! `has_server`.

pub mod boss;
pub mod dummy;
pub mod enemy;
pub mod player;
pub mod systems;

use bevy::prelude::*;

use bevymmo_shared::entity::components::SpawnPoint;
use bevymmo_shared::entity::events::{DeathEvent, RespawnedEvent};
use bevymmo_shared::network::mode::has_server;

use crate::gameplay::entity::systems::mark_dead_entities;

/// Umbrella plugin: registers all concrete entity plugins and shared server
/// systems for the entity domain.
pub struct EntityServerPlugin;

impl Plugin for EntityServerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SpawnPoint>();
        app.add_message::<DeathEvent>();
        app.add_message::<RespawnedEvent>();

        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(enemy::EnemyPlugin);
        app.add_plugins(dummy::DummyPlugin);
        app.add_plugins(boss::BossPlugin);

        // `mark_dead_entities` runs after `apply_damage` (both in FixedUpdate):
        // even if order is not strictly required thanks to the `Changed<VitalStats>`
        // filter, chaining it here avoids a tick delay in transitioning to `Dead`.
        app.add_systems(FixedUpdate, mark_dead_entities.run_if(has_server));
    }
}
