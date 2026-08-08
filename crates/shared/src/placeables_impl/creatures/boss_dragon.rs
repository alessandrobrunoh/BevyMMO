//! Dragon boss archetype.
//!
//! Reuses the canonical `Boss::SPELLS` rotation declared by the entity layer,
//! but exposes it through the placeable catalog so each boss kind can declare
//! its own rotation in the future.

use std::sync::Arc;

use crate::entity::boss::components::Boss;
use crate::placeables::{
    AssetHint, BossConfig, BossPlaceable, KindId, PlaceableDefaults, PlaceableDefinition,
    PlaceableRegistry,
};
use crate::spells::SpellId;
use crate::stats::defaults::boss_defaults;

pub struct BossDragonDefinition;

impl PlaceableDefinition for BossDragonDefinition {
    fn id(&self) -> KindId {
        KindId::new("boss_dragon")
    }
    fn display_name(&self) -> &'static str {
        "Dragon"
    }
    fn icon(&self) -> &'static str {
        "🐉"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Scene("models/boss_dragon.glb")
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults::default()
    }
}

impl BossPlaceable for BossDragonDefinition {
    fn boss_config(&self) -> BossConfig {
        BossConfig {
            stats: boss_defaults(),
            rotation: Boss::SPELLS.iter().map(|id| SpellId::new(*id)).collect(),
            arena_radius: Boss::ARENA_RADIUS,
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_boss(Arc::new(BossDragonDefinition));
}
