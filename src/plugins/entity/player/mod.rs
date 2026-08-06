//! Player: entità controllata da un client.

pub mod components;
pub mod spawn;
pub mod systems;

use bevy::prelude::*;

pub use components::Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, _app: &mut App) {
        // La transizione a `Dead` è gestita centralmente in `EntityPlugin`
        // (`mark_dead_entities`), valida per tutte le entità.
    }
}
