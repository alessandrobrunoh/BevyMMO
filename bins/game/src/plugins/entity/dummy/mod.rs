//! Dummy entity — static target for damage and UI testing.
//!
//! The Dummy is a stationary entity with massive HP, useful for testing the
//! damage system, targeting UI, colored healthbars, and spells without player
//! movement or enemy reactions.

pub mod components;
pub mod spawn;

use bevy::prelude::*;

/// Dummy Plugin: registers the bundle and EntityDefinition.
pub struct DummyPlugin;

impl Plugin for DummyPlugin {
    fn build(&self, _app: &mut App) {
        // No specific systems: the Dummy is purely a static entity
    }
}

