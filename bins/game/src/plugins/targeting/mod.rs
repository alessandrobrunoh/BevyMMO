//! Right-click targeting system with ray-sphere intersection.
//!
//! Provides:
//! - [`CurrentTarget`] resource to track the selected target
//! - Simple geometric picking system without collider dependencies
//! - Auto-cleanup of target when entity disappears or loses required components

mod plugin;
mod resources;
mod systems;

pub use plugin::TargetingPlugin;
pub use resources::CurrentTarget;
