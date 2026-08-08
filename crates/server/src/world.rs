//! Headless authored world loading.
//!
//! The server reads the same manifest as the client but never loads meshes or
//! presentation assets. Collision and authoritative world systems will build
//! on this resource in later slices.

use bevy::prelude::*;
use bevymmo_shared::paths;
use bevymmo_shared::world::{load_map_auto, CollisionGrid, MapManifest};

#[derive(Resource, Clone)]
pub struct ServerWorldMap {
    pub manifest: MapManifest,
    pub collision: CollisionGrid,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_server_map);
    }
}

fn load_server_map(mut commands: Commands) {
    let map_path = paths::default_map_file();
    match load_map_auto(&map_path) {
        Ok(manifest) => {
            let collision = CollisionGrid::build(&manifest);
            info!(
                "Loaded server map {:?} with {} props and {} collision obstacles",
                manifest.map_id,
                manifest.props.len(),
                collision.obstacle_count()
            );
            commands.insert_resource(ServerWorldMap {
                manifest,
                collision,
            });
        }
        Err(error) => {
            panic!("Unable to load server map {}: {error}", map_path.display());
        }
    }
}
