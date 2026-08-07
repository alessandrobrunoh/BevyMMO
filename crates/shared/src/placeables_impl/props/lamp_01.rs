//! Street lamp post with warm glow.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;

pub struct Lamp01Prop;

impl PlaceableDefinition for Lamp01Prop {
    fn id(&self) -> KindId { KindId::new("lamp_01") }
    fn display_name(&self) -> &'static str { "Lamp" }
    fn icon(&self) -> &'static str { "💡" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Placeholder }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.3, 1.6, 0.3],
            },
            tint: Some([0.9, 0.85, 0.5]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for Lamp01Prop {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(Lamp01Prop));
}
