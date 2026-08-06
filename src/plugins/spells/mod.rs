//! Spells framework.
//!
//! This module provides a flexible spell casting system that integrates with the
//! stats module for damage, healing, and stat modifications.

mod components;
mod context;
#[cfg(feature = "client")]
mod effects;
mod events;
mod plugin;
mod registry;
mod systems;
#[cfg(feature = "client")]
mod ui;

pub use components::{SpellCooldowns, Spellbook};
pub use context::{Spell, SpellCastContext, SpellConfig};
pub use events::SpellCastRequest;
pub use plugin::SpellsPlugin;
pub use registry::{SpellId, SpellRegistry};
pub use systems::HomingProjectile;
#[cfg(feature = "client")]
pub use ui::{SpellHudCooldownStarted, SpellHudState};
