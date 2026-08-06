//! Enemy spawn definition.
//!
//! Using `spawn_entity::<Enemy>()` creates the enemy with `GameEntity`,
//! stats, `Position`, `EntityColor`, the bundle below, and
//! `Replicate::to_clients(NetworkTarget::All)` automatically, so it is
//! immediately synchronized over the network.

use bevy::color::Color;
use bevy::prelude::*;

use super::components::{AggroRange, Enemy};
use crate::entity::definition::EntityDefinition;
use crate::spells::{HotbarSlot, SpellCooldowns, SpellHotbar, SpellId};
use crate::stats::components::StatsBundleData;

impl EntityDefinition for Enemy {
    fn name() -> &'static str {
        "Enemy"
    }

    fn bundle() -> impl Bundle {
        (
            Enemy,
            AggroRange::default(),
            {
                let mut hotbar = SpellHotbar::default();
                hotbar.assign(HotbarSlot::Q, Some(SpellId::new("attack")));
                hotbar
            },
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

    fn entity_kind() -> crate::entity::components::EntityKind {
        crate::entity::components::EntityKind::Hostile
    }
}
