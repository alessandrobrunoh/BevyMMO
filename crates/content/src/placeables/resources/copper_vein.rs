//! Copper ore vein resource node with a GLB model.

use crate::items::ItemId;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, ResourceConfig,
    ResourceNodePlaceable,
};
use crate::world::TransformData;
use std::sync::Arc;

pub struct CopperVeinDefinition;

impl PlaceableDefinition for CopperVeinDefinition {
    fn id(&self) -> KindId {
        KindId::new("resource_copper_vein")
    }
    fn display_name(&self) -> &'static str {
        "Copper Vein"
    }
    fn icon(&self) -> &'static str {
        "🪨"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Scene("models/resources/copper_vein.glb")
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            tint: Some([0.6, 0.4, 0.25]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl ResourceNodePlaceable for CopperVeinDefinition {
    fn resource_config(&self) -> ResourceConfig {
        ResourceConfig {
            max_pieces: 3,
            channel_seconds: 2.0,
            min_channel_seconds: 0.25,
            yield_item: ItemId::new("copper_ore"),
            yield_amount: 2,
            regen_interval_seconds: 30.0,
            regen_amount: 3,
            interact_range: 2.5,
            required_item_id: None,
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_resource(Arc::new(CopperVeinDefinition));
}
