//! Goblin enemy archetype.
//!
//! Frail raider: same Cleave the player sword uses, tighter aggro, leash back
//! to camp.

use std::sync::Arc;

use crate::ability_definitions::cleave::Cleave;
use crate::placeables::{
    AbilityKitEntry, AssetHint, EnemyConfig, EnemyPlaceable, KindId, PlaceableDefaults,
    PlaceableDefinition, PlaceableRegistry,
};
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
        stats.vital.current_health = 30.0;
        stats.vital.max_health = 30.0;
        stats.combat.armor = 8.0;

        EnemyConfig {
            stats,
            aggro: 8.0,
            leash_aggro: 20.0,
            abilities: vec![AbilityKitEntry::new(Cleave::ID)],
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_enemy(Arc::new(GoblinDefinition));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilityId;
    use crate::placeables::EnemyPlaceable;

    #[test]
    fn goblin_uses_the_same_cleave_as_the_sword() {
        let config = GoblinDefinition.enemy_config();
        assert_eq!(config.abilities.len(), 1);
        assert_eq!(config.abilities[0].ability_id, AbilityId::new(Cleave::ID));
        assert!(config.abilities[0].inscription.is_empty());
        assert!(!config
            .abilities
            .iter()
            .any(|entry| entry.ability_id.as_str() == "fireball"));
    }

    #[test]
    fn goblin_stats_and_leash_are_authored() {
        let config = GoblinDefinition.enemy_config();
        assert_eq!(config.stats.vital.max_health, 30.0);
        assert_eq!(config.stats.combat.armor, 8.0);
        assert_eq!(config.aggro, 8.0);
        assert_eq!(config.leash_aggro, 20.0);
    }
}
