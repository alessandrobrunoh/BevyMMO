//! Player-specific components.


/// Marker component for the player. Placed on the game entity controlled
/// by a client. Network components (`PlayerId`, `PlayerPosition`, `PlayerColor`)
/// remain in `crate::network::protocol` because they are tied to lightyear replication.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default, Clone, Copy)]
pub struct Player;
