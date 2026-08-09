//! Base scene: camera, directional light, and ground plane.
//!
//! The lifecycle is driven by [`GameScreen`]: the scene is spawned only
//! once when entering `InGame`/`Paused` and despawned (root + children)
//! when returning to `MainMenu`/`Settings`/`Connecting`. The [`GameSceneRoot`]
//! marker makes the spawn idempotent.

pub mod systems;

use bevy::prelude::*;

pub struct BaseScenePlugin;

impl Plugin for BaseScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<systems::CameraZoom>()
            .add_systems(
                Update,
                (
                    systems::update_game_scene_lifecycle,
                    systems::follow_controlled_player,
                    systems::handle_camera_zoom,
                ),
            );
    }
}
