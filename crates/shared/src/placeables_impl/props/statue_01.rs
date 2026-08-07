//! Tall marble statue.

use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;
use std::sync::Arc;

pub struct Statue01Prop;

impl PlaceableDefinition for Statue01Prop {
    fn id(&self) -> KindId {
        KindId::new("statue_01")
    }
    fn display_name(&self) -> &'static str {
        "Statue"
    }
    fn icon(&self) -> &'static str {
        "🗿"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Placeholder
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.8, 2.0, 0.8],
            },
            tint: Some([0.75, 0.75, 0.78]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for Statue01Prop {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(Statue01Prop));
}
