//! PvP zone trigger: marks the inside region as PvP-enabled.

use crate::placeables::{
    AssetHint, KindId, PlaceableDefaults, PlaceableDefinition, PlaceableRegistry, TriggerConfig,
    TriggerEvent, TriggerPlaceable, TriggerShape,
};
use crate::world::TransformData;
use std::sync::Arc;

pub struct PvpZoneTrigger;

impl PlaceableDefinition for PvpZoneTrigger {
    fn id(&self) -> KindId {
        KindId::new("trigger_pvp_zone")
    }
    fn display_name(&self) -> &'static str {
        "PvP Zone"
    }
    fn icon(&self) -> &'static str {
        "⚔️"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Invisible
    }
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

impl TriggerPlaceable for PvpZoneTrigger {
    fn trigger_config(&self) -> TriggerConfig {
        TriggerConfig {
            shape: TriggerShape::Circle { radius: 15.0 },
            event: TriggerEvent::EnterPvpZone,
            once_per_entity: false,
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_trigger(Arc::new(PvpZoneTrigger));
}
