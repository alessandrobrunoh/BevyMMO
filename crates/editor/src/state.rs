//! Editor state: tool selection, snap settings, palette, in-memory manifest.

use bevy::prelude::*;
use bevymmo_shared::world::{MapBounds, MapManifest};

/// Identifier for the active editor tool.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorTool {
    /// Click on a prop to select it.
    #[default]
    Select,
    /// Click on the ground to place the current palette kind.
    Place,
}

/// Editor's in-memory state. Persists across frames, lives in a `Resource`.
#[derive(Resource)]
pub struct EditorState {
    /// Active tool.
    pub tool: EditorTool,
    /// Currently selected entity, if any.
    pub selected: Option<Entity>,
    /// Logical kind to place (e.g. "tree_oak").
    pub current_kind: String,
    /// Current map being edited. Empty -> start a new map.
    pub manifest: MapManifest,
    /// Optional file path to save to / load from.
    pub file_path: Option<String>,
    /// Whether the manifest has unsaved edits.
    pub dirty: bool,
    /// Next id sequence for new props (e.g. "prop_0007").
    pub next_prop_seq: u32,
    /// Snap grid size in world units. Editor only — the manifest itself
    /// stores the exact authored transforms.
    pub snap_translation: f32,
    /// Camera focus point (world). Orbit camera orbits around it.
    pub camera_focus: Vec3,
    /// Camera distance from the focus point.
    pub camera_distance: f32,
    /// Camera yaw in radians.
    pub camera_yaw: f32,
    /// Camera pitch in radians (clamped to iso-friendly range).
    pub camera_pitch: f32,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            tool: EditorTool::default(),
            selected: None,
            current_kind: "cube".to_string(),
            manifest: MapManifest {
                version: bevymmo_shared::world::CURRENT_VERSION,
                map_id: "untitled".to_string(),
                display_name: "Untitled Map".to_string(),
                bounds: MapBounds {
                    min_x: -20.0,
                    max_x: 20.0,
                    min_z: -20.0,
                    max_z: 20.0,
                },
                props: Vec::new(),
            },
            file_path: None,
            dirty: false,
            next_prop_seq: 1,
            snap_translation: 1.0,
            camera_focus: Vec3::ZERO,
            camera_distance: 25.0,
            camera_yaw: 0.0,
            camera_pitch: std::f32::consts::FRAC_PI_4,
        }
    }
}

impl EditorState {
    pub fn find_prop_index(&self, id: &str) -> Option<usize> {
        self.manifest.props.iter().position(|p| p.id == id)
    }

    pub fn next_prop_id(&mut self) -> String {
        let id = format!("prop_{:04}", self.next_prop_seq);
        self.next_prop_seq += 1;
        id
    }
}

/// Marker component on every prop entity in the editor. Distinguishes editor
/// props from any other entity that may share the world.
#[derive(Component)]
pub struct EditorProp {
    /// Stable id matching the manifest entry.
    pub prop_id: String,
}

/// Marker on the selected entity's visual cue (e.g. an emissive overlay).
#[derive(Component)]
pub struct SelectedMarker;
