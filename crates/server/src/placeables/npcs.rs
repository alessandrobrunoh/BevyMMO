//! Server binding for NPC placeables (stub).
//!
//! Reads NPC placements from the manifest at map load and logs them. Full
//! interaction protocol (`InteractionRequest`/`InteractionResponse`) is a
//! future slice — this module reserves the namespace and provides the
//! extension point.

use bevy::prelude::*;

use bevymmo_shared::placeables::KindId;

/// Marker component for an NPC entity spawned from a placeable kind.
///
/// Spawned at map load by `spawn_placeables_on_map_load` (centralized in
/// [`super::creatures`]) via [`spawn_npc`]. The interaction protocol
/// (`InteractionRequest`/`InteractionResponse`) is a future slice; the marker
/// exists so those systems can target the entity once they land.
#[derive(Component, Debug, Clone)]
pub struct NpcMarker {
    /// Catalog kind that produced this NPC (e.g. `"npc_merchant"`).
    pub kind_id: KindId,
}

/// Spawns a minimal NPC marker entity at `position`.
///
/// Carries only [`NpcMarker`] + [`Transform`] + [`Name`]; no stats,
/// replication, or visual. The future interaction system will add behavior by
/// querying `NpcMarker`.
pub fn spawn_npc(commands: &mut Commands, kind_id: KindId, position: Vec3) -> Entity {
    commands
        .spawn((
            NpcMarker { kind_id: kind_id.clone() },
            Transform::from_translation(position),
            Name::new(format!("NPC {}", kind_id)),
        ))
        .id()
}

/// Plugin hook for NPC server systems.
///
/// Spawn is centralized in [`super::creatures::spawn_placeables_on_map_load`];
/// this plugin reserves the extension point for the future interaction
/// protocol.
pub struct NpcPlaceablesPlugin;

impl Plugin for NpcPlaceablesPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: register `InteractionRequest`/`InteractionResponse` messages
        // and the system that resolves `NpcPlaceable::interaction()` into the
        // appropriate replicated component.
    }
}
