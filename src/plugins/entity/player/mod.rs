//! Player: entità controllata da un client.

pub mod components;
pub mod spawn;
pub mod systems;

use bevy::prelude::*;

pub use components::Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, super::systems::despawn_dead_entities);
    }
}
