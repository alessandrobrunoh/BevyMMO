//! Events related to spell casting.

use bevy::ecs::entity::Entity;
use bevy::math::Vec3;
use bevy::prelude::Message;

use super::registry::SpellId;

/// Request to cast a spell.
///
/// This event is typically sent from clients to the server, where it is
/// processed and validated before the spell is actually cast.
#[derive(Debug, Clone, PartialEq, Message)]
pub struct SpellCastRequest {
    /// The entity attempting to cast the spell.
    pub caster: Entity,
    /// The unique identifier of the spell to cast.
    pub spell_id: SpellId,
    /// Optional target position for the spell.
    ///
    /// - Some spells may require a target position (e.g., area-of-effect spells)
    /// - Self-targeted or caster-centered spells may use None
    pub target_position: Option<Vec3>,
    /// Optional target entity for homing/projectile spells.
    pub target_entity: Option<Entity>,
}

impl SpellCastRequest {
    /// Create a new spell cast request.
    pub fn new(caster: Entity, spell_id: SpellId, target_position: Option<Vec3>) -> Self {
        Self {
            caster,
            spell_id,
            target_position,
            target_entity: None,
        }
    }

    /// Create a caster-centered spell request (no target position).
    pub fn caster_centered(caster: Entity, spell_id: SpellId) -> Self {
        Self {
            caster,
            spell_id,
            target_position: None,
            target_entity: None,
        }
    }

    /// Create a targeted spell request with a specific position.
    pub fn targeted(caster: Entity, spell_id: SpellId, target_position: Vec3) -> Self {
        Self {
            caster,
            spell_id,
            target_position: Some(target_position),
            target_entity: None,
        }
    }
}

/// Request to release a spell in cast/channel phase.
///
/// Originated from client `SpellCastRelease` command, translated here into an
/// internal event to keep spell logic protocol-independent.
#[derive(Debug, Clone, PartialEq, Message)]
pub struct SpellReleaseRequest {
    /// Caster entity requesting release.
    pub caster: Entity,
    /// Spell to release.
    pub spell_id: SpellId,
}

