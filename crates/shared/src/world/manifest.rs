//! The map manifest: the single source of truth for a map's authored content.

use serde::{Deserialize, Serialize};

use super::shapes::CollisionShape;

/// Current manifest format version. The loader rejects unknown versions.
pub const CURRENT_VERSION: u32 = 1;

/// A single authored map. Sections that are reserved for later slices are
/// always present (empty `Vec`) so the format stays stable as features land.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MapManifest {
    /// Format version. The loader rejects unknown versions.
    pub version: u32,
    /// Stable map id used for DB keys and save files (e.g. "starting_village").
    pub map_id: String,
    /// Author-facing display name.
    pub display_name: String,
    /// World size in world units (x/z). Outside bounds is not part of the map.
    pub bounds: MapBounds,
    /// Static visual props (trees, houses, rocks, ...).
    pub props: Vec<Prop>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct MapBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl MapBounds {
    pub fn contains(&self, x: f32, z: f32) -> bool {
        x >= self.min_x && x <= self.max_x && z >= self.min_z && z <= self.max_z
    }
}

/// Static object placed in the world.
///
/// `kind` is a logical content id, never a file path. The client resolves it
/// through its asset registry; the server only uses `collision`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Prop {
    /// Stable unique id within the map. Used for selection and future
    /// persistence overrides.
    pub id: String,
    /// Logical content id (e.g. "tree_oak").
    pub kind: String,
    /// Full spatial transform.
    pub transform: TransformData,
    /// Optional color tint (linear RGB, 0..1). Absent -> asset default colors.
    pub tint: Option<[f32; 3]>,
    /// Optional server-side collision shape. None -> walkable/passable.
    pub collision: Option<CollisionShape>,
    /// Whether this prop blocks movement. Server-authoritative.
    pub blocks_movement: bool,
}

/// Euler-based transform — intuitive for an editor, converted to Quat at
/// consumption time. Rotation is in degrees, YXZ order (yaw on Y).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct TransformData {
    pub translation: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
}

impl TransformData {
    /// Identity transform at the given position.
    pub fn at(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: [x, y, z],
            rotation_deg: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}
