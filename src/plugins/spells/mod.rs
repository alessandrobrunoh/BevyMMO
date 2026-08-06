//! Spells framework.
//!
//! This module provides a flexible spell casting system that integrates with the
//! stats module for damage, healing, and stat modifications.

pub mod aoe;
mod components;
pub mod context;
#[cfg(feature = "client")]
mod effects;
mod events;
mod plugin;
pub mod projectile;
mod registry;
pub mod systems;
#[cfg(feature = "client")]
pub use effects::SpellVisual;
#[cfg(feature = "client")]
mod ui;

pub use components::{SpellCooldowns, Spellbook};
pub use context::{
    AoeEffect, ProjectileSpawnRequest, Spell, SpellCastContext, SpellConfig, TargetingMode,
};
pub use events::SpellCastRequest;
pub use plugin::SpellsPlugin;
pub use projectile::HomingProjectile;
pub use registry::{SpellId, SpellRegistry};
#[cfg(feature = "client")]
pub use ui::{SpellHudCooldownStarted, SpellHudState};
