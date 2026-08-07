//! In-game map editor for BevyMMO.
//!
//! Foundation slice: 3D viewport with a ground plane, click-to-place cube,
//! click-to-select, simple inspector for transform/tint/collision, save/load
//! RON map file. The full feature set is described in `plans/map-editor.md`.

use bevy::prelude::*;

mod camera;
mod ground;
mod io;
mod picking;
mod state;
mod ui;

pub use state::EditorState;

use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default());
        app.insert_resource(EditorState::default());
        app.add_systems(Startup, (camera::spawn_camera, ground::spawn_ground));
        app.add_systems(
            Update,
            (
                picking::place_or_select,
                picking::delete_selected,
                camera::orbit_camera,
                io::save_on_ctrl_s,
                io::load_on_ctrl_o,
            ),
        );
        app.add_systems(EguiPrimaryContextPass, ui::inspector_panel);
    }
}
