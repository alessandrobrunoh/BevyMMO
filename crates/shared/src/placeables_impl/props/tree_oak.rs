//! Broadleaf oak tree with a GLB model.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, PropPlaceable,
};
use crate::world::TransformData;

pub struct TreeOakProp;

impl PlaceableDefinition for TreeOakProp {
    fn id(&self) -> KindId { KindId::new("tree_oak") }
    fn display_name(&self) -> &'static str { "Oak Tree" }
    fn icon(&self) -> &'static str { "🌳" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Scene("models/props/tree_oak.glb") }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.8, 2.5, 0.8],
            },
            tint: Some([0.2, 0.5, 0.2]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl PropPlaceable for TreeOakProp {}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_prop(Arc::new(TreeOakProp));
}
