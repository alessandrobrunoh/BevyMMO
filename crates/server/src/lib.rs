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
pub mod spells;
pub mod stats;

pub mod prelude {
    pub use crate::crowd_control::{ApplyCrowdControlEvent, CrowdControlPlugin};
    pub use crate::network::server::{
        DbPlayerId, Joined, PendingJoin, ServerConnectionConfig, ServerPlugins,
    };
    pub use crate::persistence::{
        normalize_name, PersistedPlayerSnapshot, PersistenceError, PersistencePlugin,
        PersistenceRuntime, PlayerStore,
    };
    pub use crate::stats::StatsPlugin;
}
