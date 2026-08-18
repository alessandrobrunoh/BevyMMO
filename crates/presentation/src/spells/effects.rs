//! Marker component for spell visual entities.
//!
//! [`crate::spells::cleanup_spell_visuals`] despawns every marked entity when
//! leaving gameplay. Concrete spawn/animate functions live in per-spell
//! submodules of `crate::spells`.

use bevy::prelude::*;

/// Marker placed on all spell visual entities. Enables centralized cleanup
/// when leaving gameplay.
#[derive(Component)]
pub struct SpellVisual;
