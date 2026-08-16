//! Internal ECS request to cast the equipped weapon's Eidolon gesture.
//! Mirrors `crate::spells::events::SpellCastRequest`, translated from the
//! network `EidolonCastCommand` the same way that one is translated from
//! `SpellCastCommand` (see `bevymmo_server::network::server`).

use crate::EntityId;
use glam::Vec3;

use super::slot::AbilitySlot;

#[cfg_attr(feature = "bevy", derive(bevy_ecs::message::Message))]
#[derive(Debug, Clone, PartialEq)]
pub struct EidolonCastRequest {
    pub caster: EntityId,
    pub slot: AbilitySlot,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<EntityId>,
}
