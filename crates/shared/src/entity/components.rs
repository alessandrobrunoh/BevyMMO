//! Components shared by all game entities.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Original spawn position of an entity.
///
/// Inserted at spawn time (server-side) and used by respawn systems to
/// restore the entity to its initial position. Not modified at runtime:
/// the current position lives in `Position`.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct SpawnPoint(pub Vec3);

/// Marker for any game entity (Player, Enemy, NPC, ...).
///
/// The name avoids ambiguity with `bevy::ecs::entity::Entity`, which identifies
/// an ECS instance, not a gameplay category.
#[derive(Component, Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GameEntity;

/// Shared and replicated behavioral state of a game entity.
///
/// Transitions that affect gameplay must be decided by the server.
/// `Dead` is terminal until an explicit respawn system assigns a
/// new state.
#[derive(
    Component, Debug, Default, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq,
)]
#[reflect(Component)]
pub enum EntityState {
    #[default]
    Idle,
    Moving,
    Dead,
}

impl EntityState {
    pub fn is_dead(self) -> bool {
        self == Self::Dead
    }
}

/// Player name (UI and logging only).
#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct PlayerName(pub String);

/// Game entity type used to determine alliances and targeting behavior.
///
/// Values influence UI (healthbar color, target frame), targeting rules,
/// and future interactions. The kind is replicated by the server and clients use
/// it for visual feedback, not authoritative gameplay logic.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq)]
#[reflect(Component)]
pub enum EntityKind {
    /// Local client player.
    Player,
    /// Friendly NPCs (e.g. merchants, quest givers).
    Friendly,
    /// Neutral creatures that do not attack first.
    Neutral,
    /// Hostile enemies (enemy, boss, aggressive creatures).
    Hostile,
}
