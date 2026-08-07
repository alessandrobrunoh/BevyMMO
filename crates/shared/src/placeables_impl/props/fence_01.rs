//! Wooden fence segment.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;

pub struct Fence01Prop;

impl PlaceableDefinition for Fence01Prop {
    fn id(&self) -> KindId { KindId::new("fence_01") }
    fn display_name(&self) -> &'static str { "Fence" }
    fn icon(&self) -> &'static str { "🚧" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Placeholder }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.6, 0.9, 0.2],
            },
            tint: Some([0.55, 0.4, 0.25]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for Fence01Prop {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(Fence01Prop));
}
