//! Goblin enemy archetype.
//!
//! Lower HP than the default enemy profile; chases on a tighter aggro radius.

use std::sync::Arc;

use crate::placeables::{
    AssetHint, EnemyConfig, EnemyPlaceable, KindId, PlaceableDefaults, PlaceableDefinition,
    PlaceableRegistry,
};
use crate::spells::{HotbarSlot, SpellHotbar, SpellId};
use crate::stats::defaults::enemy_defaults;

pub struct GoblinDefinition;

impl PlaceableDefinition for GoblinDefinition {
    fn id(&self) -> KindId {
        KindId::new("mob_goblin")
    }
    fn display_name(&self) -> &'static str {
        "Goblin"
    }
    fn icon(&self) -> &'static str {
        "👺"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Scene("models/creatures/goblin.glb")
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults::default()
    }
}

impl EnemyPlaceable for GoblinDefinition {
    fn enemy_config(&self) -> EnemyConfig {
        let mut stats = enemy_defaults();
        // Goblins are frail raiders.
        stats.vital.current_health = 30.0;
        stats.vital.max_health = 30.0;

        let mut spell_hotbar = SpellHotbar::default();
        spell_hotbar.assign(HotbarSlot::Q, Some(SpellId::new("fireball")));

        EnemyConfig {
            stats,
            spell_hotbar,
            aggro_range: 8.0,
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_enemy(Arc::new(GoblinDefinition));
}
