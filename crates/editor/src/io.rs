//! Save and load the in-memory manifest to/from a `.ron` file.
//!
//! Triggered by Ctrl+S (save) and Ctrl+O (load). For the foundation slice the
//! file path is taken from `EditorState::file_path`; if empty, save falls back
//! to `assets/maps/<map_id>.ron` relative to the current working directory.

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use bevymmo_shared::world::{load_map, save_map};

use crate::state::EditorState;

pub fn save_on_ctrl_s(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<EditorState>) {
    if !ctrl_held(&keys) || !just_pressed(&keys, KeyCode::KeyS) {
        return;
    }
    let path = state
        .file_path
        .clone()
        .unwrap_or_else(|| format!("assets/maps/{}.ron", state.manifest.map_id));
    match save_map(&path, &state.manifest) {
        Ok(()) => {
            info!("Saved map to {path}");
            state.file_path = Some(path);
            state.dirty = false;
        }
        Err(e) => error!("Save failed: {e}"),
    }
}

pub fn load_on_ctrl_o(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<EditorState>) {
    if !ctrl_held(&keys) || !just_pressed(&keys, KeyCode::KeyO) {
        return;
    }
    let Some(path) = state.file_path.clone() else {
        warn!("No file path set — cannot load");
        return;
    };
    match load_map(&path) {
        Ok(manifest) => {
            state.manifest = manifest;
            state.dirty = false;
            info!("Loaded map from {path}");
        }
        Err(e) => error!("Load failed: {e}"),
    }
}

fn ctrl_held(keys: &Res<ButtonInput<KeyCode>>) -> bool {
    keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)
}

fn just_pressed(keys: &Res<ButtonInput<KeyCode>>, code: KeyCode) -> bool {
    keys.just_pressed(code)
}

// Keep `Key` referenced for the future when we switch to the new keyboard API.
#[allow(dead_code)]
fn _key_keep(_k: Key) {}
