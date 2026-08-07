//! In-game map editor for BevyMMO.
//!
//! The editor provides: an orbit camera with WASD pan, Photoshop-style tool
//! strip (Select/Move/Rotate/Scale/Place/Erase), a transform gizmo on the
//! selection, a selectable terrain cube, a palette + inspector (transform,
//! tint, collision), and save/load of RON map files. The full feature set is
//! described in `plans/map-editor.md`.

use bevy::gizmos::transform_gizmo::{TransformGizmoPlugin, TransformGizmoSystems};
use bevy::prelude::*;

mod camera;
mod ground;
mod history;
mod io;
mod manipulation;
mod picking;
mod state;
mod theme;
mod ui;

pub use state::EditorState;

use bevy_egui::input::{egui_wants_any_keyboard_input, egui_wants_any_pointer_input};
use bevy_egui::{EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass};

use history::EditorHistory;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin {
            #[allow(deprecated)]
            enable_multipass_for_primary_context: false,
            ..default()
        });
        app.insert_resource(EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        });
        app.add_plugins(TransformGizmoPlugin);
        // The gizmo reads raw mouse input, so it must not react while the
        // pointer is over an egui panel.
        app.configure_sets(
            PostUpdate,
            TransformGizmoSystems.run_if(not(egui_wants_any_pointer_input)),
        );
        app.insert_resource(EditorState::default());
        app.insert_resource(EditorHistory::default());
        app.insert_resource(picking::PickBodyCache::default());
        app.insert_resource(picking::PropMeshRegistry::default());
        app.add_systems(
            Startup,
            (
                camera::spawn_camera,
                ground::spawn_ground,
                ui::spawn_native_hud,
            ),
        );

        // World-input systems: anything driven by the mouse or the cursor.
        app.add_systems(
            Update,
            (
                picking::refresh_pick_cache,
                picking::place_or_select,
                picking::update_hover,
                picking::rebuild_scene.after(io::load_on_ctrl_o),
                camera::orbit_camera,
                ground::draw_grid,
                manipulation::sync_gizmo_settings,
                manipulation::sync_gizmo_focus,
                manipulation::record_gizmo_drag_start,
                manipulation::writeback_manifest,
                manipulation::draw_overlays,
                io::recompute_validation,
            )
                .chain()
                .run_if(not(egui_wants_any_pointer_input)),
        );

        // Keyboard-only systems run even while the pointer hovers a panel,
        // but not while egui is typing in a text field.
        app.add_systems(
            Update,
            (
                camera::keyboard_pan,
                picking::delete_selected,
                picking::deselect_on_escape,
                io::save_on_ctrl_s,
                io::load_on_ctrl_o,
                io::new_map_on_ctrl_n,
                io::undo_redo,
                io::duplicate_on_ctrl_d,
                ui::keyboard_tools,
            )
                .run_if(not(egui_wants_any_keyboard_input)),
        );

        app.add_systems(EguiPrimaryContextPass, ui::inspector_panel);
    }
}
