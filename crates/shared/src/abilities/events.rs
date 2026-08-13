//! Internal ECS request to cast the equipped weapon's Eidolon gesture.
//! Mirrors `crate::spells::events::SpellCastRequest`, translated from the
//! network `EidolonCastCommand` the same way that one is translated from
//! `SpellCastCommand` (see `bevymmo_server::network::server`).

use bevy::ecs::entity::Entity;
use bevy::math::Vec3;
use bevy::prelude::Message;

use super::slot::AbilitySlot;

#[derive(Debug, Clone, PartialEq, Message)]
pub struct EidolonCastRequest {
    pub caster: Entity,
    pub slot: AbilitySlot,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<Entity>,
}
