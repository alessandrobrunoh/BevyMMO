//! Base scene: camera, directional light, and ground plane.
//!
//! The lifecycle is driven by [`Screen`]: the scene is spawned on
//! `OnEnter(Screen::InGame)` and despawned on `OnExit`. Pause is a client
//! overlay and does not despawn the scene. The [`GameSceneRoot`] marker is a
//! safety check against a duplicate spawn.
//!
//! The [`occlusion`] submodule hides the canopy/roof of props (any glTF node
//! whose name ends with `_Top`) when it blocks the line of sight between the
//! game camera and the locally controlled player.

pub mod occlusion;
pub mod systems;

use bevy::prelude::*;

use crate::game_state::{Screen, not_typing};
use crate::renderer::RenderSync;

pub struct BaseScenePlugin;

impl Plugin for BaseScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<systems::CameraZoom>()
            .add_systems(OnEnter(Screen::InGame), systems::spawn_game_scene)
            .add_systems(OnExit(Screen::InGame), systems::despawn_game_scene)
            .add_systems(
                Update,
                (
                    // Previously ungated entirely — not even by screen — so
                    // PageUp/PageDown zoomed the (invisible, not-yet-spawned)
                    // camera from the main menu, and would also have fired
                    // while typing.
                    systems::handle_camera_zoom
                        .run_if(in_state(Screen::InGame))
                        .run_if(not_typing),
                    occlusion::tag_occludables,
                    occlusion::update_camera_occlusion.run_if(in_state(Screen::InGame)),
                    occlusion::animate_occluder_fade.run_if(in_state(Screen::InGame)),
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
