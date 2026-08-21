//! Internal request to cast an equipped `BaseAbility` (`cast_weapon`).

use crate::EntityId;
use glam::Vec3;

use super::slot::AbilitySlot;

#[cfg_attr(feature = "bevy", derive(bevy_ecs::message::Message))]
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponCastRequest {
    pub caster: EntityId,
    pub slot: AbilitySlot,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<EntityId>,
}
