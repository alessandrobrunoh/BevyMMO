//! Editor state: tool selection, snap settings, palette, in-memory manifest.

use bevy::gizmos::transform_gizmo::{TransformGizmoMode, TransformGizmoSpace};
use bevy::prelude::*;
use bevymmo_shared::world::{MapBounds, MapManifest};

/// Identifier for the active editor tool.
///
/// `Move`/`Rotate`/`Scale` drive the built-in transform gizmo; `Select` and
/// `Place` work directly on the viewport; `Erase` removes a prop on click.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorTool {
    /// Click on a prop or the terrain to select it.
    #[default]
    Select,
    /// Show the translate gizmo on the selection.
    Move,
    /// Show the rotate gizmo on the selection.
    Rotate,
    /// Show the scale gizmo on the selection.
    Scale,
    /// Click on the ground to place the current palette kind.
    Place,
    /// Click on a prop to delete it.
    Erase,
}

impl EditorTool {
    /// All tools, in toolbar display order.
    pub const ALL: [EditorTool; 6] = [
        EditorTool::Select,
        EditorTool::Move,
        EditorTool::Rotate,
        EditorTool::Scale,
        EditorTool::Place,
        EditorTool::Erase,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EditorTool::Select => "Select",
            EditorTool::Move => "Move",
            EditorTool::Rotate => "Rotate",
            EditorTool::Scale => "Scale",
            EditorTool::Place => "Place",
            EditorTool::Erase => "Erase",
        }
    }

    pub fn hotkey(self) -> &'static str {
        match self {
            EditorTool::Select => "V",
            EditorTool::Move => "W",
            EditorTool::Rotate => "E",
            EditorTool::Scale => "R",
            EditorTool::Place => "B",
            EditorTool::Erase => "X",
        }
    }
}

/// Editor's in-memory state. Persists across frames, lives in a `Resource`.
#[derive(Resource)]
pub struct EditorState {
    /// Active tool.
    pub tool: EditorTool,
    /// Currently selected entity (prop or terrain), if any.
    pub selected: Option<Entity>,
    /// Entity under the cursor (hover feedback), if any.
    pub hovered: Option<Entity>,
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
    /// Snap step for gizmo rotation, in degrees.
    pub snap_rotation_deg: f32,
    /// Snap step for gizmo scaling.
    pub snap_scale: f32,
    /// Whether the gizmo manipulates in world or local space.
    pub gizmo_space: TransformGizmoSpace,
    /// Whether the placement/snap grid overlay is visible.
    pub show_grid: bool,
    /// Set after a load to force the scene to be rebuilt from the manifest.
    pub needs_rebuild: bool,
    /// Entity id of the terrain cube, so picking and rebuilds can find it.
    pub terrain_entity: Option<Entity>,
    /// Camera focus point (world). Orbit camera orbits around it.
    pub camera_focus: Vec3,
    /// Camera distance from the focus point.
    pub camera_distance: f32,
    /// Camera yaw in radians.
    pub camera_yaw: f32,
    /// Camera pitch in radians (clamped to iso-friendly range).
    pub camera_pitch: f32,
    /// Whether destructive actions (delete) should show a confirmation prompt.
    pub confirm_delete: bool,
    /// Tracks the previous-frame gizmo activation so a single undo snapshot
    /// is taken when a drag *starts* (not on every pixel of the drag).
    pub gizmo_was_active: bool,
    /// Cached validation issues for the current manifest. Refreshed on demand
    /// by the `recompute_validation` system so the UI never blocks the frame.
    pub validation_issues: Vec<bevymmo_shared::world::ValidationIssue>,
    /// Whether the validation cache needs to be recomputed this frame.
    pub validation_dirty: bool,
    /// Currently selected left-panel tab. Pure UI state.
    pub left_tab: LeftPanelTab,
    /// Filter text for the palette tab. Empty -> show all kinds.
    pub palette_search: String,
    /// Set by the menu bar to request a duplicate of the selection. Consumed
    /// by `io::duplicate_on_ctrl_d` so the hotkey and the menu stay in sync.
    pub pending_duplicate: bool,
    /// Set by the menu bar to request a focus-on-selection. Consumed by the
    /// camera system so the action survives between frames.
    pub pending_focus_selection: bool,
    /// Set to true by the Delete button when `confirm_delete` is on. The
    /// inspector renders the modal in a separate pass so the user can confirm
    /// or cancel without the action happening in the same frame as the click.
    pub pending_delete_dialog: bool,
}

/// Left-panel tab identifier.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftPanelTab {
    /// Hierarchy of placed props + terrain.
    #[default]
    Outliner,
    /// Palette of placeable kinds, grouped by category.
    Palette,
    /// Snap settings (translation / rotation / scale steps, gizmo space).
    Snap,
    /// Map metadata (id, name, bounds).
    MapSettings,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            tool: EditorTool::default(),
            selected: None,
            hovered: None,
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
                terrain: bevymmo_shared::world::Terrain::default(),
                props: Vec::new(),
            },
            file_path: None,
            dirty: false,
            next_prop_seq: 1,
            snap_translation: 1.0,
            snap_rotation_deg: 15.0,
            snap_scale: 0.1,
            gizmo_space: TransformGizmoSpace::World,
            show_grid: true,
            needs_rebuild: false,
            terrain_entity: None,
            camera_focus: Vec3::ZERO,
            camera_distance: 25.0,
            camera_yaw: 0.0,
            camera_pitch: std::f32::consts::FRAC_PI_4,
            confirm_delete: true,
            gizmo_was_active: false,
            validation_issues: Vec::new(),
            validation_dirty: true,
            left_tab: LeftPanelTab::Outliner,
            palette_search: String::new(),
            pending_duplicate: false,
            pending_focus_selection: false,
            pending_delete_dialog: false,
        }
    }
}

impl EditorState {
    /// The gizmo manipulation mode for the active tool, if any.
    pub fn gizmo_mode(&self) -> Option<TransformGizmoMode> {
        match self.tool {
            EditorTool::Move => Some(TransformGizmoMode::Translate),
            EditorTool::Rotate => Some(TransformGizmoMode::Rotate),
            EditorTool::Scale => Some(TransformGizmoMode::Scale),
            _ => None,
        }
    }

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

/// Marker on the terrain cube entity.
#[derive(Component)]
pub struct EditorTerrain;

/// Marker on the selected entity's visual cue (e.g. an emissive overlay).
#[derive(Component)]
pub struct SelectedMarker;

/// Converts the manifest's YXZ euler degrees into a Bevy quaternion.
///
/// The manifest stores rotation as `[pitch, yaw, roll]` degrees applied in
/// YXZ order (yaw around Y), matching the client's consumption code.
pub fn quat_from_rotation_deg(rotation_deg: [f32; 3]) -> Quat {
    Quat::from_euler(
        EulerRot::YXZ,
        rotation_deg[1].to_radians(),
        rotation_deg[0].to_radians(),
        rotation_deg[2].to_radians(),
    )
}
