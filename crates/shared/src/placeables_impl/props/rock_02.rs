//! Small weathered stone.

use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;
use std::sync::Arc;

pub struct Rock02Prop;

impl PlaceableDefinition for Rock02Prop {
    fn id(&self) -> KindId {
        KindId::new("rock_02")
    }
    fn display_name(&self) -> &'static str {
        "Rock (Small)"
    }
    fn icon(&self) -> &'static str {
        "🪨"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Placeholder
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.9, 0.6, 0.8],
            },
            tint: Some([0.45, 0.42, 0.38]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for Rock02Prop {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(Rock02Prop));
}
