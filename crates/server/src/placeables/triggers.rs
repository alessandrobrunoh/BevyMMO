//! Server binding for trigger zone placeables (stub).
//!
//! The full `evaluate_triggers` system (proximity detection, `TriggerEvent`
//! dispatch, one-shot bookkeeping) is a future slice. This module reserves the
//! namespace and provides the extension point.

use bevy::prelude::*;

use bevymmo_shared::placeables::KindId;

/// Marker component for a trigger zone spawned from a placeable kind.
#[derive(Component, Debug, Clone)]
pub struct TriggerMarker {
    /// Catalog kind that produced this trigger (e.g. `"trigger_pvp_zone"`).
    pub kind_id: KindId,
}

/// Spawns a minimal trigger zone marker entity at `position`.
///
/// Triggers are invisible zones, so the entity carries only [`TriggerMarker`]
/// + [`Transform`] + [`Name`]; no visual. The future `evaluate_triggers`
/// system will query `TriggerMarker` and resolve `trigger_config()`.
pub fn spawn_trigger(commands: &mut Commands, kind_id: KindId, position: Vec3) -> Entity {
    commands
        .spawn((
            TriggerMarker { kind_id: kind_id.clone() },
            Transform::from_translation(position),
            Name::new(format!("Trigger {}", kind_id)),
        ))
        .id()
}

/// Plugin hook for trigger server systems.
///
/// Spawn is centralized in [`super::creatures::spawn_placeables_on_map_load`];
/// this plugin reserves the extension point for the future
/// `evaluate_triggers` system.
pub struct TriggerPlaceablesPlugin;

impl Plugin for TriggerPlaceablesPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: `evaluate_triggers` system that walks `TriggerMarker` entities,
        // resolves `TriggerPlaceable::trigger_config()`, and applies
        // `TriggerEvent` (PvP/Safe/Teleport) to entities inside the shape.
    }
}
