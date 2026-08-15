//! Spawn definition for the dragon boss.
//!
//! Using `spawn_entity::<Boss>()` applies `GameEntity`, stats, `Position`,
//! `EntityColor`, the bundle below and `Replicate::to_clients(NetworkTarget::All)`
//! automatically, so the boss is immediately synchronized over the network.

use bevy::color::Color;
use bevy::prelude::{Bundle, Vec3};

use super::components::{
    Boss, BossArena, BossPhase, BossRotationState, BossSpellbook, ThreatTable,
};
use crate::entity::components::EntityKind;
use crate::entity::definition::EntityDefinition;
use crate::spells::{SpellCooldowns, SpellId};


impl EntityDefinition for Boss {
    fn name() -> &'static str {
        "Boss"
    }

    fn bundle() -> impl Bundle {
        (
            Boss,
            BossPhase::Dormant,
            BossArena {
                center: Self::initial_position(),
                radius: Self::ARENA_RADIUS,
                is_engaged: false,
            },
            ThreatTable::default(),
            BossSpellbook {
                spells: Boss::SPELLS.iter().map(|id| SpellId::new(*id)).collect(),
            },
            BossRotationState::default(),
            SpellCooldowns::default(),
        )
    }

    fn initial_position() -> Vec3 {
        Vec3::new(0.0, 0.0, -12.0)
    }

    fn initial_color() -> Color {
        // Ashen red: the dragon sleeps as a glowing crimson silhouette.
        Color::srgb(0.55, 0.05, 0.05)
    }

    fn stats() -> crate::stats::components::StatsBundleData {
        crate::stats::defaults::boss_defaults()
    }

    fn entity_kind() -> EntityKind {
        EntityKind::Hostile
    }
}
