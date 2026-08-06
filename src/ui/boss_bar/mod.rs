//! Boss health bar + phase banner UI.
//!
//! Reads the boss's replicated `VitalStats`, `BossArena` and `BossPhase`. The bar
//! shows only while the encounter is engaged and the boss is alive. A transient
//! phase banner pops up for ~2s whenever `BossPhase` changes, so transitions are
//! readable without any extra server->client message.

pub mod components;
pub mod plugin;
pub mod systems;

pub use plugin::BossBarPlugin;
