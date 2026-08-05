//! Definizione di spawn del Player.
//!
//! `bundle()` contiene solo il marker `Player`. Le componenti generiche
//! (`Position`, `EntityColor`, `Health`) sono gestite centralmente.
//! Il player ha comunque network custom (prediction/interpolation
//! dipendenti dall'owner) gestita in `network::server::handle_connected_client`.

use bevy::prelude::*;

use super::components::Player;
use crate::plugins::entity::components::{Health, Stats};
use crate::plugins::entity::definition::EntityDefinition;

impl EntityDefinition for Player {
    fn name() -> &'static str {
        "Player"
    }

    fn bundle() -> impl Bundle {
        (Player,)
    }

    fn initial_color() -> Color {
        Color::srgb(0.2, 0.8, 0.2)
    }

    fn health() -> Health {
        Health::new(100.0)
    }

    fn stats() -> Stats {
        Stats::with_combat_values(0.15, 10.0, 100.0, 100.0, 5.0, 25.0)
    }
}
