//! Server-authoritative player-specific systems and plugin.

use bevy::prelude::*;

pub mod systems;

/// Player plugin (server side).
///
/// Transition to `Dead` is managed centrally in `EntityServerPlugin`
/// (`mark_dead_entities`), applicable to all entities.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, _app: &mut App) {}
}
