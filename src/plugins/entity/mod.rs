//! Entity plugin — container per tutte le entità di gioco (Player, Enemy, ...).
//!
//! Ogni entità concreta vive in un proprio sotto-modulo e registra il proprio
//! `EntityPlugin` figlio. Il plugin "padre" `EntityPlugin` raccoglie i figli
//! ed espone i tipi condivisi (`GameEntity`, stato, nome, ...).

pub mod components;
pub mod definition;
pub mod dummy;
pub mod events;
pub mod spawn;
pub mod systems;

pub mod enemy;
pub mod player;

use bevy::prelude::*;

use crate::network::mode::has_server;

use components::SpawnPoint;
use events::{DeathEvent, RespawnedEvent};

/// Plugin padre: registra tutti i plugin delle entità concrete.
pub struct EntityPlugin;

impl Plugin for EntityPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SpawnPoint>();
        app.add_message::<DeathEvent>();
        app.add_message::<RespawnedEvent>();

        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(enemy::EnemyPlugin);
        app.add_plugins(dummy::DummyPlugin);

        // `mark_dead_entities` gira dopo `apply_damage` (entrambi in FixedUpdate):
        // anche se l'ordine non è strettamente richiesto grazie al filtro
        // `Changed<VitalStats>`, chainarlo qui evita un tick di ritardo nella
        // transizione a `Dead`.
        app.add_systems(FixedUpdate, systems::mark_dead_entities.run_if(has_server));
    }
}
