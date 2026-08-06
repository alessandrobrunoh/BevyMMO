//! Entity plugin — container for all game entities (Player, Enemy, ...).
//!
//! Each concrete entity lives in its own sub-module and registers its own
//! child `EntityPlugin`. The parent `EntityPlugin` gathers the children
//! and exposes shared types (`GameEntity`, state, name, ...).

pub mod components;
pub mod definition;
pub mod dummy;
pub mod events;
pub mod spawn;
pub mod systems;

pub mod boss;
pub mod enemy;
pub mod player;

use bevy::prelude::*;

use crate::network::mode::has_server;

use components::SpawnPoint;
use events::{DeathEvent, RespawnedEvent};

/// Parent plugin: registers all concrete entity plugins.
pub struct EntityPlugin;

impl Plugin for EntityPlugin {
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
        app.add_systems(FixedUpdate, systems::mark_dead_entities.run_if(has_server));
    }
}
