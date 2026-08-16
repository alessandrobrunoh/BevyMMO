//! Greeter NPC with a dialogue interaction.

use std::sync::Arc;

use crate::placeables::{
    AssetHint, InteractionKind, KindId, NpcPlaceable, PlaceableDefaults, PlaceableDefinition,
    PlaceableRegistry,
};
use crate::world::TransformData;

pub struct GreeterDefinition;

impl PlaceableDefinition for GreeterDefinition {
    fn id(&self) -> KindId {
        KindId::new("npc_greeter")
    }
    fn display_name(&self) -> &'static str {
        "Greeter"
    }
    fn icon(&self) -> &'static str {
        "👋"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Placeholder
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            tint: None,
            collision: None,
            blocks_movement: false,
        }
    }
}

impl NpcPlaceable for GreeterDefinition {
    fn interaction(&self) -> InteractionKind {
        InteractionKind::Dialogue {
            dialogue_tree_id: "greeting".to_string(),
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_npc(Arc::new(GreeterDefinition));
}
