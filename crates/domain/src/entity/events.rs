//! Gameplay events related to entity lifecycle.
//!
//! These are local ECS messages (not network messages): emitted by server-side systems that
//! mutate life state (`mark_dead_entities`, `handle_respawn_request`,
//! `enemy_respawn`) so downstream systems can react
//! (logging, loot drops, achievements, audio, ...).


use crate::EntityId;
use super::components::EntityKind;

/// Emitted when an entity transitions into `EntityState::Dead`.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::message::Message))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeathEvent {
    pub entity: EntityId,
    pub kind: EntityKind,
}

/// Emitted when an entity is brought back from `Dead` to a living state.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::message::Message))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RespawnedEvent {
    pub entity: EntityId,
}
