//! In-game map editor for BevyMMO.
//!
//! This is currently a thin placeholder plugin that proves the workspace can
//! start an editor-specific mode without the client or server transport.
//! The full data-driven editor is described in `plans/map-editor.md`.

use bevy::prelude::*;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, announce_editor_mode);
    }
}

fn announce_editor_mode() {
    info!("Editor mode placeholder started. Full implementation is planned in plans/map-editor.md");
}
