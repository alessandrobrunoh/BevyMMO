//! Dummy spawn definition.
//!
//! The Dummy is a static entity with massive HP, used for testing
//! the damage system, targeting UI, and spells. It has no AI and does not move.

use bevy::color::Color;
use bevy::prelude::*;

use super::components::Dummy;
use crate::entity::components::PlayerName;
use crate::entity::definition::EntityDefinition;
use crate::stats::components::StatsBundleData;

impl EntityDefinition for Dummy {
    fn name() -> &'static str {
        "Dummy"
    }

    fn bundle() -> impl Bundle {
        (Dummy, PlayerName("Dummy".to_string()))
    }

    fn initial_position() -> Vec3 {
        Vec3::new(8.0, 0.0, 0.0)
    }

    fn initial_color() -> Color {
        Color::srgb(0.7, 0.1, 0.1)
    }

    fn entity_kind() -> crate::entity::components::EntityKind {
        crate::entity::components::EntityKind::Hostile
    }

    fn stats() -> StatsBundleData {
        crate::stats::defaults::dummy_defaults()
    }
}
