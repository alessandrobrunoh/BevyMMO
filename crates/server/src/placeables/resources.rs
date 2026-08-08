//! Server binding for resource node placeables (stub).
//!
//! The full gathering system (`Harvestable` health depletion, yield roll,
//! respawn timer) is a future slice. This module reserves the namespace and
//! the marker so future work can target it without re-editing the catalog.

use bevy::prelude::*;

use bevymmo_shared::placeables::{KindId, ResourceConfig};

/// Marker + state for a harvestable resource node.
///
/// Spawned at map load by `spawn_placeables_on_map_load` (centralized in
/// [`super::creatures`]) via [`spawn_resource_node`], which initializes
/// `current_health` from [`ResourceConfig::max_health`]. The gathering loop is
/// a future slice; the component exists so that system can target it.
#[derive(Component, Debug, Clone)]
pub struct Harvestable {
    /// Catalog kind that produced this node (e.g. `"resource_copper_vein"`).
    pub kind_id: KindId,
    /// Current health; when it reaches 0 the node depletes.
    pub current_health: f32,
    /// Respawn countdown (seconds) once depleted; `0.0` means ready.
    pub respawn_remaining: f32,
}

/// Spawns a minimal resource node marker entity at `position`, seeding
/// [`Harvestable`] state from the kind's [`ResourceConfig`].
///
/// Carries only `Harvestable` + [`Transform`] + [`Name`]; no stats,
/// replication, or visual. The future gathering system will query
/// `Harvestable` to deplete, yield, and respawn.
pub fn spawn_resource_node(
    commands: &mut Commands,
    kind_id: KindId,
    position: Vec3,
    config: ResourceConfig,
) -> Entity {
    commands
        .spawn((
            Harvestable {
                kind_id: kind_id.clone(),
                current_health: config.max_health,
                respawn_remaining: 0.0,
            },
            Transform::from_translation(position),
            Name::new(format!("Resource {}", kind_id)),
        ))
        .id()
}

/// Plugin hook for resource node server systems.
///
/// Spawn is centralized in [`super::creatures::spawn_placeables_on_map_load`];
/// this plugin reserves the extension point for the future gathering system.
pub struct ResourcePlaceablesPlugin;

impl Plugin for ResourcePlaceablesPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: gathering system that decrements `Harvestable::current_health`
        // on harvest input, yields `ResourceConfig::yield_item` on depletion,
        // and starts the `respawn_seconds` timer.
    }
}
