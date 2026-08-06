//! Definizione di spawn dell'Enemy.
//!
//! Usando `spawn_entity::<Enemy>()` l'enemy viene creato con `GameEntity`,
//! statistiche, `Position`, `EntityColor`, il bundle qui sotto e
//! `Replicate::to_clients(NetworkTarget::All)` in automatico, quindi è
//! subito sincronizzato sul network.

use bevy::color::Color;
use bevy::prelude::*;

use super::components::{AggroRange, Enemy};
use crate::plugins::entity::definition::EntityDefinition;
use crate::plugins::spells::{SpellCooldowns, SpellId, Spellbook};
use crate::stats::components::StatsBundleData;

impl EntityDefinition for Enemy {
    fn name() -> &'static str {
        "Enemy"
    }

    fn bundle() -> impl Bundle {
        (
            Enemy,
            AggroRange::default(),
            Spellbook::single(SpellId::new("attack")),
            SpellCooldowns::default(),
        )
    }

    fn initial_position() -> Vec3 {
        Vec3::new(5.0, 0.0, 5.0)
    }

    fn initial_color() -> Color {
        Color::srgb(0.8, 0.2, 0.2)
    }

    fn stats() -> StatsBundleData {
        crate::stats::defaults::enemy_defaults()
    }

    fn entity_kind() -> crate::plugins::entity::components::EntityKind {
        crate::plugins::entity::components::EntityKind::Hostile
    }
}
