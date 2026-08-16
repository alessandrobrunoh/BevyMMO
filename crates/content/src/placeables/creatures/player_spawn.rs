//! Invisible player spawn marker.
//!
//! Placed in the manifest as a `Creature`-category prop; the server records
//! its position into a `PlayerSpawnPoints` resource instead of spawning a
//! `Player` entity (players are spawned on client join).

use std::sync::Arc;

use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry,
    PlayerSpawnPlaceable,
};

pub struct PlayerSpawnDefinition;

impl PlaceableDefinition for PlayerSpawnDefinition {
    fn id(&self) -> KindId {
        KindId::new("player_spawn")
    }
    fn display_name(&self) -> &'static str {
        "Player Spawn"
    }
    fn icon(&self) -> &'static str {
        "✦"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Invisible
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults::default()
    }
}

impl PlayerSpawnPlaceable for PlayerSpawnDefinition {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_player_spawn(Arc::new(PlayerSpawnDefinition));
}
