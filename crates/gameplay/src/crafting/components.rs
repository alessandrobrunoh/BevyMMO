//! Runtime components mirrored onto a character that is crafting.

#[cfg(feature = "bevy")]
use bevy_ecs::component::Component;

use crate::items::ItemId;

/// Local player's (or another character's) in-progress craft channel.
#[cfg_attr(feature = "bevy", derive(Component))]
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveCraft {
    pub npc_entity_id: u64,
    pub item_id: ItemId,
    pub quantity: u32,
    pub elapsed_seconds: f32,
    pub required_seconds: f32,
}
