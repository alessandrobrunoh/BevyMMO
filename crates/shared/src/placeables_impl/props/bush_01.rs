//! Low decorative bush.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;

pub struct Bush01Prop;

impl PlaceableDefinition for Bush01Prop {
    fn id(&self) -> KindId { KindId::new("bush_01") }
    fn display_name(&self) -> &'static str { "Bush" }
    fn icon(&self) -> &'static str { "🌿" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Placeholder }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.8, 0.7, 0.8],
            },
            tint: Some([0.25, 0.45, 0.2]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for Bush01Prop {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(Bush01Prop));
}
