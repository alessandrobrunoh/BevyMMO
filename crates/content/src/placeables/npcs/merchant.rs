//! General-purpose merchant NPC with a shop interaction.

use std::sync::Arc;

use crate::placeables::{
    AssetHint, InteractionKind, KindId, NpcPlaceable, PlaceableDefaults, PlaceableDefinition,
    PlaceableRegistry,
};
use crate::world::TransformData;

pub struct MerchantDefinition;

impl PlaceableDefinition for MerchantDefinition {
    fn id(&self) -> KindId {
        KindId::new("npc_merchant")
    }
    fn display_name(&self) -> &'static str {
        "Merchant"
    }
    fn icon(&self) -> &'static str {
        "🧑‍💼"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Scene("models/npcs/merchant.glb")
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            tint: Some([0.6, 0.5, 0.9]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl NpcPlaceable for MerchantDefinition {
    fn interaction(&self) -> InteractionKind {
        InteractionKind::Shop {
            inventory_id: "shop_general".to_string(),
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_npc(Arc::new(MerchantDefinition));
}
