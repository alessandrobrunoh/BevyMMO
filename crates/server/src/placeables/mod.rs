//! Server-side placeable catalog wiring.
//!
//! Owns the run-once map-load pass that turns creature placements in the
//! manifest into live gameplay entities (enemies, bosses) or recorded spawn
//! points (player spawns). The `PlaceableRegistry` itself is populated by the
//! `register_server_placeables` `Startup` system declared in `ServerPlugin`.

pub mod creatures;
pub mod interactables;
pub mod npcs;
pub mod persistence;
pub mod resources;
pub mod triggers;

use bevy::prelude::*;

use bevymmo_shared::network::mode::has_server;

use crate::placeables::creatures::{spawn_placeables_on_map_load, PlaceablesSpawned, PlayerSpawnPoints};
use crate::placeables::persistence::{apply_prop_overrides_on_map_load, PropOverridesApplied};
use crate::persistence::PersistenceRuntime;
use crate::world::ServerWorldMap;

/// Wires server-only placeable systems.
pub struct PlaceablesPlugin;

impl Plugin for PlaceablesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSpawnPoints>();

        // Run-once map-load pass. Placed on `Update` (not `Startup`) so it runs
        // the first frame AFTER `WorldPlugin::load_server_map` has inserted
        // `ServerWorldMap`. The `PlaceablesSpawned` guard resource guarantees it
        // executes exactly once. This decouples us from cross-plugin `Startup`
        // ordering while still completing before the first gameplay tick.
        app.add_systems(
            Update,
            apply_prop_overrides_on_map_load
                .before(spawn_placeables_on_map_load)
                .run_if(has_server)
                .run_if(resource_exists::<ServerWorldMap>)
                .run_if(resource_exists::<PersistenceRuntime>)
                .run_if(not(resource_exists::<PropOverridesApplied>)),
        );

        app.add_systems(
            Update,
            spawn_placeables_on_map_load
                .run_if(has_server)
                .run_if(resource_exists::<ServerWorldMap>)
                .run_if(not(resource_exists::<PlaceablesSpawned>)),
        );
    }
}
