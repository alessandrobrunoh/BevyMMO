//! Save / load / new map actions plus undo/redo wiring.
//!
//! The keyboard-driven systems delegate to pure helper functions
//! ([`save`], [`load`], [`new_map`], [`undo`], [`redo`]) so the egui menu bar
//! can call exactly the same logic without duplicating it. Every destructive
//! action pushes the *previous* manifest onto [`EditorHistory`] before
//! mutating, so undo can restore the exact pre-action state.

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use bevymmo_shared::world::{
    load_map, save_map, validate, MapBounds, MapManifest, Terrain, CURRENT_VERSION,
};

use crate::history::EditorHistory;
use crate::state::{EditorProp, EditorState};

/// Default file used by `Ctrl+S` / `Ctrl+O` when no explicit path is set.
fn default_path(state: &EditorState) -> String {
    format!("assets/maps/{}.ron", state.manifest.map_id)
}

/// Saves the current manifest. Returns an error string on failure so callers
/// (hotkey system or menu bar) can surface it consistently.
pub fn save(state: &mut EditorState) -> Result<(), String> {
    let path = state
        .file_path
        .clone()
        .unwrap_or_else(|| default_path(state));
    save_map(&path, &state.manifest)
        .map(|()| {
            state.file_path = Some(path.clone());
            state.dirty = false;
            info!("Saved map to {path}");
        })
        .map_err(|e| {
            error!("Save failed: {e}");
            e.to_string()
        })
}

/// Loads the manifest from disk, resets history and triggers a rebuild.
pub fn load(state: &mut EditorState, history: &mut EditorHistory) -> Result<(), String> {
    let path = state
        .file_path
        .clone()
        .unwrap_or_else(|| default_path(state));
    match load_map(&path) {
        Ok(manifest) => {
            state.manifest = manifest;
            state.dirty = false;
            state.next_prop_seq = state.manifest.props.len() as u32 + 1;
            state.needs_rebuild = true;
            state.validation_dirty = true;
            history.clear();
            info!("Loaded map from {path}");
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            error!("Load failed: {e}");
            Err(msg)
        }
    }
}

/// Replaces the in-memory manifest with a fresh default and triggers a
/// scene rebuild. The previous map is *not* saved automatically.
pub fn new_map(state: &mut EditorState, history: &mut EditorHistory) {
    history.push(&state.manifest);
    state.manifest = MapManifest {
        version: CURRENT_VERSION,
        map_id: "untitled".to_string(),
        display_name: "Untitled Map".to_string(),
        bounds: MapBounds {
            min_x: -20.0,
            max_x: 20.0,
            min_z: -20.0,
            max_z: 20.0,
        },
        terrain: Terrain::default(),
        props: Vec::new(),
    };
    state.file_path = None;
    state.dirty = true;
    state.next_prop_seq = 1;
    state.selected = None;
    state.hovered = None;
    state.needs_rebuild = true;
    state.validation_dirty = true;
    info!("Created new map");
}

/// Restores the previous manifest snapshot, if any.
pub fn undo(state: &mut EditorState, history: &mut EditorHistory) -> bool {
    if let Some(manifest) = history.undo(&state.manifest) {
        apply_restored(state, manifest);
        true
    } else {
        false
    }
}

/// Re-applies the next redo snapshot, if any.
pub fn redo(state: &mut EditorState, history: &mut EditorHistory) -> bool {
    if let Some(manifest) = history.redo(&state.manifest) {
        apply_restored(state, manifest);
        true
    } else {
        false
    }
}

fn apply_restored(state: &mut EditorState, manifest: MapManifest) {
    state.manifest = manifest;
    state.next_prop_seq = state.manifest.props.len() as u32 + 1;
    state.selected = None;
    state.hovered = None;
    state.needs_rebuild = true;
    state.validation_dirty = true;
    state.dirty = true;
}

/// Recomputes the cached validation issues when `validation_dirty` is set.
/// Kept separate from the UI pass so a slow validation rule can never block
/// the frame.
pub fn recompute_validation(mut state: ResMut<EditorState>) {
    if !state.validation_dirty {
        return;
    }
    state.validation_issues = validate(&state.manifest);
    state.validation_dirty = false;
}

// ---------------------------------------------------------------------------
// Keyboard-driven systems. They are thin wrappers around the pure helpers so
// the menu bar can call the exact same code paths.
// ---------------------------------------------------------------------------

/// `Ctrl+S`.
pub fn save_on_ctrl_s(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<EditorState>) {
    if ctrl_held(&keys) && just_pressed(&keys, KeyCode::KeyS) {
        let _ = save(&mut state);
    }
}

/// `Ctrl+O`.
pub fn load_on_ctrl_o(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<EditorState>,
    mut history: ResMut<EditorHistory>,
) {
    if ctrl_held(&keys) && just_pressed(&keys, KeyCode::KeyO) {
        let _ = load(&mut state, &mut history);
    }
}

/// `Ctrl+N`.
pub fn new_map_on_ctrl_n(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<EditorState>,
    mut history: ResMut<EditorHistory>,
) {
    if ctrl_held(&keys) && just_pressed(&keys, KeyCode::KeyN) {
        new_map(&mut state, &mut history);
    }
}

/// `Ctrl+Z` (undo) and `Ctrl+Y` / `Ctrl+Shift+Z` (redo).
pub fn undo_redo(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<EditorState>,
    mut history: ResMut<EditorHistory>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let is_undo = ctrl_held(&keys) && just_pressed(&keys, KeyCode::KeyZ) && !shift;
    let is_redo = ctrl_held(&keys)
        && (just_pressed(&keys, KeyCode::KeyY) || (just_pressed(&keys, KeyCode::KeyZ) && shift));
    if is_undo {
        undo(&mut state, &mut history);
    } else if is_redo {
        redo(&mut state, &mut history);
    }
}

/// `Ctrl+D`. Duplicates the selected prop, offsetting it by one snap step on
/// X so the user immediately sees the new prop without overlap. Reads
/// `state.pending_duplicate` so the menu bar can request the same action.
pub fn duplicate_on_ctrl_d(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<EditorState>,
    mut history: ResMut<EditorHistory>,
    prop_q: Query<(&EditorProp, &Transform), Without<crate::state::EditorTerrain>>,
    selected_q: Query<Entity, With<crate::state::SelectedMarker>>,
) {
    let requested =
        (ctrl_held(&keys) && just_pressed(&keys, KeyCode::KeyD)) || state.pending_duplicate;
    if !requested {
        return;
    }
    state.pending_duplicate = false;
    run_duplicate(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut state,
        &mut history,
        &prop_q,
        &selected_q,
    );
}

/// Pure duplicate logic, separated from the system so a future script/batch
/// path can call it.
#[allow(clippy::too_many_arguments)]
pub fn run_duplicate(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    state: &mut EditorState,
    history: &mut EditorHistory,
    prop_q: &Query<(&EditorProp, &Transform), Without<crate::state::EditorTerrain>>,
    selected_q: &Query<Entity, With<crate::state::SelectedMarker>>,
) {
    use crate::picking::spawn_prop_entity;
    use bevymmo_shared::world::{Prop, TransformData};

    let Some(selected) = state.selected else {
        return;
    };
    let Ok((source_prop, _source_transform)) = prop_q.get(selected) else {
        return;
    };
    let Some(source_index) = state.find_prop_index(&source_prop.prop_id) else {
        return;
    };

    history.push(&state.manifest);

    let source = state.manifest.props[source_index].clone();
    let new_id = state.next_prop_id();
    let offset_x = state.snap_translation.max(0.25);
    let mut new_translation = source.transform.translation;
    new_translation[0] += offset_x;
    let new_prop = Prop {
        id: new_id,
        kind: source.kind.clone(),
        transform: TransformData {
            translation: new_translation,
            rotation_deg: source.transform.rotation_deg,
            scale: source.transform.scale,
        },
        tint: source.tint,
        collision: source.collision,
        blocks_movement: source.blocks_movement,
    };

    let entity = spawn_prop_entity(commands, meshes, materials, &new_prop);
    state.manifest.props.push(new_prop);
    state.dirty = true;
    state.validation_dirty = true;

    crate::picking::clear_selection(commands, selected_q);
    commands.entity(entity).insert(crate::state::SelectedMarker);
    state.selected = Some(entity);
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
