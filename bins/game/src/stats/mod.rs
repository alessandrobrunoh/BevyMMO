//! Game stats module.
//!
//! Owns the runtime components ([`components`]), events
//! ([`events`]), formulas ([`formulas`]), modifiers ([`modifiers`]),
//! systems ([`systems`]), and default profiles ([`defaults`]).
//!
//! The plugin registers everything needed to apply damage, healing, and
//! modifiers in a server-authoritative manner.

pub mod components;
pub mod defaults;
pub mod events;
pub mod formulas;
pub mod modifiers;
pub mod systems;

pub use plugin::StatsPlugin;

mod plugin;
