//! Generic unit cube for prototyping layouts.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;

pub struct CubeProp;

impl PlaceableDefinition for CubeProp {
    fn id(&self) -> KindId { KindId::new("cube") }
    fn display_name(&self) -> &'static str { "Cube" }
    fn icon(&self) -> &'static str { "▢" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Placeholder }
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

impl PropPlaceable for CubeProp {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(CubeProp));
}
