//! Treasure chest that rolls a loot table on open.

use crate::placeables::{
    AssetHint, InteractablePlaceable, InteractionKind, KindId, PlaceableDefaults,
    PlaceableDefinition, PlaceableRegistry,
};
use crate::world::TransformData;
use std::sync::Arc;

pub struct TreasureChestInteractable;

impl PlaceableDefinition for TreasureChestInteractable {
    fn id(&self) -> KindId {
        KindId::new("interactable_treasure_chest")
    }
    fn display_name(&self) -> &'static str {
        "Treasure Chest"
    }
    fn icon(&self) -> &'static str {
        "🧰"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Scene("models/interactables/treasure_chest.glb")
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            tint: Some([0.7, 0.55, 0.2]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl InteractablePlaceable for TreasureChestInteractable {
    fn interaction(&self) -> InteractionKind {
        InteractionKind::OpenChest {
            loot_table_id: "loot_chest_basic".to_string(),
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_interactable(Arc::new(TreasureChestInteractable));
}
