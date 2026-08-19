//! Training dummy: hittable, no AI, no spells.

use std::sync::Arc;

use crate::placeables::{
    AssetHint, DummyPlaceable, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry,
};
use crate::stats::components::StatsBundleData;
use crate::stats::defaults::dummy_defaults;

pub struct DummyDefinition;

impl PlaceableDefinition for DummyDefinition {
    fn id(&self) -> KindId {
        KindId::new("training_dummy")
    }
    fn display_name(&self) -> &'static str {
        "Dummy"
    }
    fn icon(&self) -> &'static str {
        "🎯"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Placeholder
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults::default()
    }
}

impl DummyPlaceable for DummyDefinition {
    fn dummy_stats(&self) -> StatsBundleData {
        dummy_defaults()
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_dummy(Arc::new(DummyDefinition));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::dummy::components::DUMMY_RESPAWN_SECONDS;

    #[test]
    fn dummy_is_a_stationary_sack_of_hp() {
        let stats = DummyDefinition.dummy_stats();
        assert_eq!(stats.vital.max_health, 10_000.0);
        assert_eq!(stats.vital.current_health, 10_000.0);
        assert_eq!(stats.movement.speed, 0.0);
        assert_eq!(stats.combat.attack_power, 0.0);
    }

    #[test]
    fn dummy_returns_after_ten_seconds() {
        assert_eq!(DUMMY_RESPAWN_SECONDS, 10.0);
    }
}
