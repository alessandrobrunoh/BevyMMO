//! Components specific to the Dummy entity.

/// Marker component to identify a Dummy entity.
///
/// The Dummy is a static target with a large HP pool, used for testing
/// damage systems, targeting UI, and spells. It has no AI, does not move,
/// and has no spellbook.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default, Clone, Copy)]
pub struct Dummy;

/// Seconds a dead dummy stays down before standing back up.
pub const DUMMY_RESPAWN_SECONDS: f32 = 10.0;
