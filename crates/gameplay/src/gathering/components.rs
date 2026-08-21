//! Runtime components mirrored onto gatherable entities.

#[cfg(feature = "bevy")]
use bevy_ecs::component::Component;

/// Authoritative harvest state replicated onto the mirrored node entity.
#[cfg_attr(feature = "bevy", derive(Component))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Harvestable {
    pub placement_id: String,
    pub kind_id: String,
    pub current_pieces: u32,
}

/// Local player's (or another character's) in-progress gather channel.
#[cfg_attr(feature = "bevy", derive(Component))]
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveGather {
    pub node_entity_id: u64,
    pub elapsed_seconds: f32,
    pub required_seconds: f32,
}
