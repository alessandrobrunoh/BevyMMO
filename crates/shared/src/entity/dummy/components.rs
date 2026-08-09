//! Components specific to the Dummy entity.

use bevy::prelude::*;

/// Marker component to identify a Dummy entity.
///
/// The Dummy is a static target with massive HP, used for testing
/// damage systems, targeting UI, and spells. It has no AI, does not move,
/// and has no spellbook.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Dummy;
