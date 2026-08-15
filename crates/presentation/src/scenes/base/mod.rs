//! Base scene: camera, directional light, and ground plane.
//!
//! The lifecycle is driven by [`GameScreen`]: the scene is spawned only
//! once when entering `InGame`/`Paused` and despawned (root + children)
//! when returning to `MainMenu`/`Settings`/`Connecting`. The [`GameSceneRoot`]
//! marker makes the spawn idempotent.
//!
//! The [`occlusion`] submodule hides the canopy/roof of props (any glTF node
//! whose name ends with `_Top`) when it blocks the line of sight between the
//! game camera and the locally controlled player.

pub mod occlusion;
pub mod systems;

use bevy::prelude::*;

use crate::game_state::{GameScreen, Screen};
use crate::renderer::RenderSync;

pub struct BaseScenePlugin;

impl Plugin for BaseScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<systems::CameraZoom>()
            .add_systems(
                Update,
                (
                    systems::update_game_scene_lifecycle,
                    systems::handle_camera_zoom,
                    occlusion::tag_occludables,
                    occlusion::update_camera_occlusion.run_if(in_game_or_paused),
                    occlusion::animate_occluder_fade.run_if(in_game_or_paused),
                ),
            )
            // The follow must observe the transform written by
            // `RenderSync::Transforms` this frame, not the one left over from
            // the last: an out-of-date anchor shifts the whole world by a frame
            // of player movement, which is exactly the shake that only appeared
            // while walking. `handle_camera_zoom` stays unordered — it writes a
            // resource the follow reads, and a frame of latency on a keypress is
            // not observable.
            .add_systems(
                Update,
                systems::follow_controlled_player.in_set(RenderSync::Camera),
            );
    }
}

fn in_game_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}
