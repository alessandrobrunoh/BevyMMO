//! Spells framework.
//!
//! This module provides a flexible spell casting system that integrates with the
//! stats module for damage, healing, and stat modifications.

mod components;
mod context;
mod effects;
mod events;
mod plugin;
mod registry;
mod systems;
mod ui;

pub use components::{SpellCooldowns, Spellbook};
pub use context::{Spell, SpellCastContext, SpellConfig};
pub use events::SpellCastRequest;
pub use plugin::SpellsPlugin;
pub use registry::{SpellId, SpellRegistry};
pub use systems::HomingProjectile;
pub use ui::{SpellHudCooldownStarted, SpellHudState};
