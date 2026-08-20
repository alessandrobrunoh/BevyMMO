//! Allied training dummy: healable, no AI, no spells.

use std::sync::Arc;

use crate::placeables::{
    AssetHint, DummyPlaceable, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry,
};
use crate::stats::components::StatsBundleData;
use crate::stats::defaults::dummy_defaults;

pub struct AllyDummyDefinition;

impl PlaceableDefinition for AllyDummyDefinition {
    fn id(&self) -> KindId {
        KindId::new("ally_dummy")
    }
    fn display_name(&self) -> &'static str {
        "Ally Dummy"
    }
    fn icon(&self) -> &'static str {
        "💚"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Placeholder
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults::default()
    }
}

impl DummyPlaceable for AllyDummyDefinition {
    fn dummy_stats(&self) -> StatsBundleData {
        dummy_defaults()
    }

    fn is_ally(&self) -> bool {
        true
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_dummy(Arc::new(AllyDummyDefinition));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ally_dummy_is_a_healable_sack_of_hp() {
        let stats = AllyDummyDefinition.dummy_stats();
        assert_eq!(stats.vital.max_health, 10_000.0);
        assert!(AllyDummyDefinition.is_ally());
        assert_eq!(AllyDummyDefinition.id().as_str(), "ally_dummy");
    }
}
