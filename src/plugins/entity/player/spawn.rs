//! Definizione di spawn del Player.
//!
//! `bundle()` contiene solo il marker `Player`. Le componenti generiche
//! (`Position`, `EntityColor`, statistiche) sono gestite centralmente.
//! Il player ha comunque network custom (prediction/interpolation
//! dipendenti dall'owner) gestita in `network::server::handle_connected_client`.

use bevy::color::Color;
use bevy::prelude::*;

use super::components::Player;
use crate::plugins::entity::definition::EntityDefinition;
use crate::plugins::spells::{default_player_hotbar, SpellCooldowns};
use crate::stats::components::StatsBundleData;

/// Posizione iniziale del Player. Usata sia da `initial_position()` sia dal
/// sistema di respawn lato server per riportare il player in vita.
pub const PLAYER_SPAWN_POINT: Vec3 = Vec3::ZERO;

impl EntityDefinition for Player {
    fn name() -> &'static str {
        "Player"
    }

    fn bundle() -> impl Bundle {
        (Player, default_player_hotbar(), SpellCooldowns::default())
    }

    fn initial_position() -> Vec3 {
        PLAYER_SPAWN_POINT
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
