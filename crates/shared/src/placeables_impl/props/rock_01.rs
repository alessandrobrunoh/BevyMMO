//! Large grey boulder.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;

pub struct Rock01Prop;

impl PlaceableDefinition for Rock01Prop {
    fn id(&self) -> KindId { KindId::new("rock_01") }
    fn display_name(&self) -> &'static str { "Rock (Large)" }
    fn icon(&self) -> &'static str { "🪨" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Placeholder }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.4, 0.8, 1.2],
            },
            tint: Some([0.5, 0.5, 0.5]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for Rock01Prop {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(Rock01Prop));
}
