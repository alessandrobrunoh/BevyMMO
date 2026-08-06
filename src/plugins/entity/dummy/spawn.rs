//! Definizione di spawn del Dummy.
//!
//! Il Dummy è un'entità statica con HP enormi, usata per testare
//! il sistema danni, UI targeting e spell. Non ha AI e non si muove.

use bevy::color::Color;
use bevy::prelude::*;

use super::components::Dummy;
use crate::plugins::entity::components::PlayerName;
use crate::plugins::entity::definition::EntityDefinition;
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

    fn entity_kind() -> crate::plugins::entity::components::EntityKind {
        crate::plugins::entity::components::EntityKind::Hostile
    }

    fn stats() -> StatsBundleData {
        crate::stats::defaults::dummy_defaults()
    }
}
