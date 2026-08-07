//! Undo / redo history for the editor.
//!
//! Uses a coarse manifest-snapshot strategy: every user action that mutates
//! the map (place, delete, duplicate, gizmo drag, inspector edit) calls
//! [`EditorHistory::push`] *before* mutating, storing the previous manifest.
//! Undo restores the last snapshot; redo replays the next one.
//!
//! This is intentionally simple (no command pattern, no per-field diffs):
//! manifests are small (hundreds of props at most), so cloning on each
//! committed action is cheap and keeps the logic bug-free.

use std::collections::VecDeque;

use bevy::prelude::*;

use bevymmo_shared::world::MapManifest;

/// Maximum snapshots retained per side. Older entries are dropped FIFO so
/// memory stays bounded even on long editing sessions.
const MAX_HISTORY: usize = 64;

/// Undo / redo stacks. Lives as a `Resource` next to `EditorState`.
#[derive(Resource, Default)]
pub struct EditorHistory {
    undo: VecDeque<MapManifest>,
    redo: VecDeque<MapManifest>,
}

impl EditorHistory {
    /// Returns whether at least one action can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Returns whether at least one action can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Records the pre-action manifest so the action can be undone later.
    /// The redo stack is cleared: any redo branch is invalidated by a new edit.
    pub fn push(&mut self, manifest: &MapManifest) {
        if self.undo.len() >= MAX_HISTORY {
            self.undo.pop_front();
        }
        self.undo.push_back(manifest.clone());
        self.redo.clear();
    }

    /// Pops the last pre-action snapshot and pushes the current manifest onto
    /// the redo stack. Returns `None` when there is nothing to undo.
    pub fn undo(&mut self, current: &MapManifest) -> Option<MapManifest> {
        let previous = self.undo.pop_back()?;
        self.push_redo(current);
        Some(previous)
    }

    /// Re-applies the next redo snapshot, pushing the current manifest onto
    /// the undo stack. Returns `None` when there is nothing to redo.
    pub fn redo(&mut self, current: &MapManifest) -> Option<MapManifest> {
        let next = self.redo.pop_back()?;
        if self.undo.len() >= MAX_HISTORY {
            self.undo.pop_front();
        }
        self.undo.push_back(current.clone());
        Some(next)
    }

    /// Wipes both stacks. Called after load/new to avoid restoring stale maps.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    fn push_redo(&mut self, manifest: &MapManifest) {
        if self.redo.len() >= MAX_HISTORY {
            self.redo.pop_front();
        }
        self.redo.push_back(manifest.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_shared::world::{MapBounds, MapManifest, Prop, TransformData};

    fn fresh_map(name: &str) -> MapManifest {
        MapManifest {
            version: bevymmo_shared::world::CURRENT_VERSION,
            map_id: name.into(),
            display_name: name.into(),
            bounds: MapBounds {
                min_x: -10.0,
                max_x: 10.0,
                min_z: -10.0,
                max_z: 10.0,
            },
            terrain: bevymmo_shared::world::Terrain::default(),
            props: Vec::new(),
        }
    }

    fn map_with_prop(name: &str, id: &str) -> MapManifest {
        let mut m = fresh_map(name);
        m.props.push(Prop {
            id: id.into(),
            kind: "cube".into(),
            transform: TransformData::at(0.0, 0.0, 0.0),
            tint: None,
            collision: None,
            blocks_movement: false,
        });
        m
    }

    #[test]
    fn undo_restores_previous_manifest_and_clears_redo_on_new_edit() {
        let mut history = EditorHistory::default();
        let v0 = fresh_map("v0");
        let v1 = map_with_prop("v1", "prop_0001");

        // push() stores the *pre-action* state, so a single push before the
        // edit is enough to undo back to v0.
        history.push(&v0);

        let restored = history.undo(&v1).expect("undo should restore v0");
        assert_eq!(restored, v0);
        assert!(history.can_redo());

        // A new edit invalidates the redo branch.
        let v2 = map_with_prop("v2", "prop_0002");
        history.push(&v2);
        assert!(!history.can_redo());
    }

    #[test]
    fn redo_replays_undone_state() {
        let mut history = EditorHistory::default();
        let v0 = fresh_map("v0");
        let v1 = map_with_prop("v1", "prop_0001");

        history.push(&v0);
        let restored = history.undo(&v1).expect("undo present");
        assert_eq!(restored, v0);

        let redone = history.redo(&v0).expect("redo present");
        assert_eq!(redone, v1);
        assert!(!history.can_redo());
    }

    #[test]
    fn history_caps_at_max_entries() {
        let mut history = EditorHistory::default();
        for i in 0..200 {
            history.push(&fresh_map(&format!("m{i}")));
        }
        // Cannot undo infinitely; should stop after MAX_HISTORY undos.
        let mut current = fresh_map("current");
        for _ in 0..MAX_HISTORY {
            assert!(
                history.undo(&current).is_some(),
                "undo should still be available"
            );
            current = fresh_map("post-undo");
        }
        assert!(
            history.undo(&current).is_none(),
            "undo stack should be exhausted"
        );
    }

    #[test]
    fn clear_wipes_both_stacks() {
        let mut history = EditorHistory::default();
        history.push(&fresh_map("a"));
        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }
}
