//! Components specific to the Dummy entity.

/// Marker component to identify a Dummy entity.
///
/// The Dummy is a static target with massive HP, used for testing
/// damage systems, targeting UI, and spells. It has no AI, does not move,
/// and has no spellbook.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default, Clone, Copy)]
pub struct Dummy;
