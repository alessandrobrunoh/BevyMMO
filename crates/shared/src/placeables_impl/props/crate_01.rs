//! Stackable wooden crate.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;

pub struct Crate01Prop;

impl PlaceableDefinition for Crate01Prop {
    fn id(&self) -> KindId { KindId::new("crate_01") }
    fn display_name(&self) -> &'static str { "Crate" }
    fn icon(&self) -> &'static str { "📦" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Placeholder }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.6, 0.6, 0.6],
            },
            tint: Some([0.6, 0.45, 0.3]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for Crate01Prop {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(Crate01Prop));
}
