//! Headless authoritative server logic for BevyMMO.
//!
//! Hosts the Lightyear server transport, PostgreSQL persistence, and the
//! server-authoritative gameplay systems (movement simulation, AI, spells,
//! crowd control, death/respawn).
//!
//! This crate must never depend on `client` or `presentation`: the production
//! server build excludes rendering and UI entirely.

pub mod crowd_control;
pub mod gameplay;
pub mod migrations;
pub mod network;
pub mod persistence;
pub mod player_movement;
pub mod spells;
pub mod stats;
pub mod world;

use bevy::prelude::*;
use core::time::Duration;
use std::net::SocketAddr;

/// Umbrella plugin for the authoritative server runtime.
pub struct ServerPlugin {
    pub database_url: String,
    pub server_addr: SocketAddr,
    pub tick_duration: Duration,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(persistence::PersistencePlugin::new(self.database_url.clone()));
        app.add_plugins(network::server::ServerPlugins {
            server_addr: self.server_addr,
            tick_duration: self.tick_duration,
        });
        app.add_plugins((
            stats::StatsPlugin,
            gameplay::entity::EntityServerPlugin,
            player_movement::PlayerMovementPlugin,
            crowd_control::CrowdControlPlugin,
            spells::SpellsServerPlugin,
            world::WorldPlugin,
        ));
    }
}

pub mod prelude {
    pub use crate::ServerPlugin;
    pub use crate::crowd_control::{ApplyCrowdControlEvent, CrowdControlPlugin};
    pub use crate::gameplay::entity::EntityServerPlugin;
    pub use crate::player_movement::PlayerMovementPlugin;
    pub use crate::spells::SpellsServerPlugin;
    pub use crate::network::server::{
        DbPlayerId, Joined, PendingJoin, ServerConnectionConfig, ServerPlugins,
    };
    pub use crate::persistence::{
        normalize_name, PersistedPlayerSnapshot, PersistenceError, PersistencePlugin,
        PersistenceRuntime, PlayerStore,
    };
    pub use crate::stats::StatsPlugin;
}
