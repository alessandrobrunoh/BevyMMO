//! Server binding for interactable placeables (stub).
//!
//! The full interaction logic (door open/close state, chest loot roll) is a
//! future slice. This module reserves the namespace and the marker.

use bevy::prelude::*;

use bevymmo_shared::placeables::KindId;

/// Marker component for an interactable spawned from a placeable kind.
#[derive(Component, Debug, Clone)]
pub struct InteractableMarker {
    /// Catalog kind that produced this interactable (e.g.
    /// `"interactable_wooden_door"`).
    pub kind_id: KindId,
    /// Whether a one-shot interactable (chest) has already been used.
    pub consumed: bool,
}

/// Spawns a minimal interactable marker entity at `position`.
///
/// Carries only [`InteractableMarker`] (with `consumed: false`) + [`Transform`]
/// + [`Name`]; no stats, replication, or visual. The future interaction system
///   will query `InteractableMarker` to resolve effects.
pub fn spawn_interactable(commands: &mut Commands, kind_id: KindId, position: Vec3) -> Entity {
    commands
        .spawn((
            InteractableMarker {
                kind_id: kind_id.clone(),
                consumed: false,
            },
            Transform::from_translation(position),
            Name::new(format!("Interactable {}", kind_id)),
        ))
        .id()
}

/// Plugin hook for interactable server systems.
///
/// Spawn is centralized in [`super::creatures::spawn_placeables_on_map_load`];
/// this plugin reserves the extension point for the future interaction system.
pub struct InteractablePlaceablesPlugin;

impl Plugin for InteractablePlaceablesPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: interaction system that resolves
        // `InteractablePlaceable::interaction()` and applies the effect
        // (toggle door state, roll `loot_table_id`, mark `consumed`).
    }
}
