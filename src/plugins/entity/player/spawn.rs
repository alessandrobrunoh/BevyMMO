//! Definizione di spawn del Player.
//!
//! `bundle()` contiene solo il marker `Player`. Le componenti generiche
//! (`Position`, `EntityColor`, statistiche) sono gestite centralmente.
//! Il player ha comunque network custom (prediction/interpolation
//! dipendenti dall'owner) gestita in `network::server::handle_connected_client`.

use bevy::prelude::*;

use super::components::Player;
use crate::plugins::entity::definition::EntityDefinition;
use crate::plugins::spells::{SpellCooldowns, SpellId, Spellbook};
use crate::stats::components::StatsBundleData;

impl EntityDefinition for Player {
    fn name() -> &'static str {
        "Player"
    }

    fn bundle() -> impl Bundle {
        (
            Player,
            Spellbook::from_ids([SpellId::new("attack"), SpellId::new("fireball")]),
            SpellCooldowns::default(),
        )
    }

    fn initial_color() -> Color {
        Color::srgb(0.2, 0.8, 0.2)
    }

    fn stats() -> StatsBundleData {
        crate::stats::defaults::player_defaults()
    }

    fn entity_kind() -> crate::plugins::entity::components::EntityKind {
        crate::plugins::entity::components::EntityKind::Player
    }
}
