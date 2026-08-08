//! Wooden door that toggles open / closed on use.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, InteractablePlaceable, InteractionKind, KindId, PlaceableDefaults,
    PlaceableDefinition, PlaceableRegistry,
};
use crate::world::TransformData;

pub struct WoodenDoorInteractable;

impl PlaceableDefinition for WoodenDoorInteractable {
    fn id(&self) -> KindId { KindId::new("interactable_wooden_door") }
    fn display_name(&self) -> &'static str { "Wooden Door" }
    fn icon(&self) -> &'static str { "🚪" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Scene("models/interactables/wooden_door.glb") }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            tint: Some([0.55, 0.35, 0.2]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl InteractablePlaceable for WoodenDoorInteractable {
    fn interaction(&self) -> InteractionKind { InteractionKind::OpenDoor }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_interactable(Arc::new(WoodenDoorInteractable));
}
