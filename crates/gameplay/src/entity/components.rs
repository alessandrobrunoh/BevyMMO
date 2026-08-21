//! Components shared by all game entities.

// `#[reflect(Component)]` expands to a reference to this type.
#[cfg(feature = "bevy")]
use bevy_ecs::reflect::ReflectComponent;

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Original spawn position of an entity.
///
/// Inserted at spawn time (server-side) and used by respawn systems to
/// restore the entity to its initial position. Not modified at runtime:
/// the current position lives in `Position`.
#[cfg_attr(
    feature = "bevy",
    derive(bevy_ecs::component::Component, bevy_reflect::Reflect)
)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct SpawnPoint(pub Vec3);

/// Marker for any game entity (Player, Enemy, NPC, ...).
///
/// The name avoids ambiguity with `bevy::ecs::entity::EntityId`, which identifies
/// an ECS instance, not a gameplay category.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GameEntity;

/// Shared and replicated behavioral state of a game entity.
///
/// Transitions that affect gameplay must be decided by the server.
/// `Dead` is terminal until an explicit respawn system assigns a
/// new state.
#[cfg_attr(
    feature = "bevy",
    derive(bevy_ecs::component::Component, bevy_reflect::Reflect)
)]
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", reflect(Component))]
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
#[cfg_attr(
    feature = "bevy",
    derive(bevy_ecs::component::Component, bevy_reflect::Reflect)
)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct PlayerName(pub String);

/// Game entity type used to determine alliances and targeting behavior.
///
/// Values influence UI (healthbar color, target frame), targeting rules,
/// and future interactions. The kind is replicated by the server and clients use
/// it for visual feedback, not authoritative gameplay logic.
#[cfg_attr(
    feature = "bevy",
    derive(bevy_ecs::component::Component, bevy_reflect::Reflect)
)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum EntityKind {
    /// Local client player.
    Player,
    /// Friendly NPCs (e.g. merchants, quest givers).
    Friendly,
    /// Combat ally that is not an NPC (party-heal dummy, future pets).
    Ally,
    /// Neutral creatures that do not attack first.
    Neutral,
    /// Hostile enemies (enemy, boss, aggressive creatures).
    Hostile,
    /// Harvestable world node.
    Resource,
}
