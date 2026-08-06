//! Spells framework.
//!
//! This module provides a flexible spell casting system that integrates with the
//! stats module for damage, healing, and stat modifications.

pub mod aoe;
#[cfg(feature = "client")]
pub mod cast_bar;
pub mod components;
pub mod context;
#[cfg(feature = "client")]
mod effects;
mod events;
mod plugin;
pub mod projectile;
mod registry;
pub mod systems;
#[cfg(feature = "client")]
mod ui;

#[cfg(feature = "client")]
pub use effects::SpellVisual;

pub use components::{
    default_player_hotbar, CastProgress, HotbarSlot, SpellCooldowns, SpellHotbar,
};
pub use context::{
    AoeEffect, AoeTargeting, CastKind, ChannelMovementPolicy, ProjectileSpawnRequest, Spell,
    SpellCastContext, SpellConfig, TargetingMode,
};
// Re-exports dei tipi stats usati comunemente nelle spell definition.
pub use crate::stats::events::{ModifierEffect, ModifierKind};
pub use events::{SpellCastRequest, SpellReleaseRequest};
pub use plugin::SpellsPlugin;
pub use projectile::HomingProjectile;
pub use registry::{SpellId, SpellRegistry};
#[cfg(feature = "client")]
pub use ui::{SpellHudCooldownStarted, SpellHudState};
