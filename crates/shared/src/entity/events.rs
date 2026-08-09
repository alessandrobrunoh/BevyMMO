//! Gameplay events related to entity lifecycle.
//!
//! These are local ECS messages (not network messages): emitted by server-side systems that
//! mutate life state (`mark_dead_entities`, `handle_respawn_request`,
//! `enemy_respawn`) so downstream systems can react
//! (logging, loot drops, achievements, audio, ...).

use bevy::ecs::entity::Entity;
use bevy::prelude::Message;

use super::components::EntityKind;

/// Emitted when an entity transitions into `EntityState::Dead`.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct DeathEvent {
    pub entity: Entity,
    pub kind: EntityKind,
}

/// Emitted when an entity is brought back from `Dead` to a living state.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct RespawnedEvent {
    pub entity: Entity,
}
