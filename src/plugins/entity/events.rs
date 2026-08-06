//! Eventi di gameplay legati al ciclo di vita delle entità.
//!
//! Sono messaggi ECS locali (non network): emessi dai sistemi server-side che
//! mutano lo stato di vita (`mark_dead_entities`, `handle_respawn_request`,
//! `enemy_respawn`) per consentire a sistemi downstream di reagire
//! (logging, drop loot, achievement, audio, ...).

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
