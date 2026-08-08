//! Teleport trigger: moves the entering entity to another map / position.

use std::sync::Arc;
use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry,
    TriggerConfig, TriggerEvent, TriggerPlaceable, TriggerShape,
};
use crate::world::TransformData;

pub struct TeleportTrigger;

impl PlaceableDefinition for TeleportTrigger {
    fn id(&self) -> KindId { KindId::new("trigger_teleport") }
    fn display_name(&self) -> &'static str { "Teleport" }
    fn icon(&self) -> &'static str { "🌀" }
    fn asset_hint(&self) -> AssetHint { AssetHint::Invisible }
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

impl TriggerPlaceable for TeleportTrigger {
    fn trigger_config(&self) -> TriggerConfig {
        TriggerConfig {
            shape: TriggerShape::Circle { radius: 2.0 },
            event: TriggerEvent::Teleport {
                target_map: "test_1".to_string(),
                target_position: [0.0, 0.0, 0.0],
            },
            once_per_entity: true,
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_trigger(Arc::new(TeleportTrigger));
}
