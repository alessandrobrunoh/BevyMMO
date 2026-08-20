//! Isolated player-market NPCs.

use std::sync::Arc;

use crate::placeables::{
    AssetHint, InteractionKind, KindId, NpcPlaceable, PlaceableDefaults, PlaceableDefinition,
    PlaceableRegistry,
};
use crate::world::TransformData;
use bevymmo_gameplay::markets::{MARKET_1_ID, MARKET_2_ID};

struct MarketNpc {
    kind: &'static str,
    name: &'static str,
    market_id: &'static str,
}

impl PlaceableDefinition for MarketNpc {
    fn id(&self) -> KindId {
        KindId::new(self.kind)
    }
    fn display_name(&self) -> &'static str {
        self.name
    }
    fn icon(&self) -> &'static str {
        "🏦"
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
            tint: Some([0.75, 0.6, 0.25]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl NpcPlaceable for MarketNpc {
    fn interaction(&self) -> InteractionKind {
        InteractionKind::Market {
            market_id: self.market_id.to_string(),
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_npc(Arc::new(MarketNpc {
        kind: "npc_market_1",
        name: "Market 1",
        market_id: MARKET_1_ID,
    }));
    registry.register_npc(Arc::new(MarketNpc {
        kind: "npc_market_2",
        name: "Market 2",
        market_id: MARKET_2_ID,
    }));
}
