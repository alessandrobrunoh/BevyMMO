//! Simple house shell.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;

pub struct HouseSimpleProp;

impl PlaceableDefinition for HouseSimpleProp {
    fn id(&self) -> KindId { KindId::new("house_simple") }
    fn display_name(&self) -> &'static str { "House" }
    fn icon(&self) -> &'static str { "🏠" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Placeholder }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [3.0, 2.0, 3.0],
            },
            tint: Some([0.7, 0.6, 0.4]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for HouseSimpleProp {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(HouseSimpleProp));
}
