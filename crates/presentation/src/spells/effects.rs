//! Marker component for spell visual entities.
//!
//! Centralized cleanup systems (in the binary) query this marker to despawn
//! every visual effect when leaving gameplay. Concrete visual spawn/animate
//! functions live in per-spell submodules of `crate::spells`.

use bevy::prelude::*;

/// Marker placed on all spell visual entities. Enables centralized cleanup
/// when leaving gameplay.
#[derive(Component)]
pub struct SpellVisual;
