//! Player-specific components.

use bevy::prelude::*;

/// Marker component for the player. Placed on the game entity controlled
/// by a client. Network components (`PlayerId`, `PlayerPosition`, `PlayerColor`)
/// remain in `crate::network::protocol` because they are tied to lightyear replication.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Player;
