//! Player: client-controlled entity.

pub mod components;
pub mod spawn;
pub mod systems;

use bevy::prelude::*;

pub use components::Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, _app: &mut App) {
        // Transition to `Dead` is managed centrally in `EntityPlugin`
        // (`mark_dead_entities`), applicable to all entities.
    }
}

