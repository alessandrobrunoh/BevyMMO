//! The map manifest: the single source of truth for a map's authored content.

use serde::{Deserialize, Serialize};

use super::shapes::CollisionShape;
use crate::placeables::KindId;

/// Current manifest format version. The loader rejects unknown versions.
pub const CURRENT_VERSION: u32 = 2;

/// Legacy version 1 format constant for backward compatibility checks.
pub const LEGACY_VERSION_1: u32 = 1;

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
    /// The ground cube. Present in every map; authors can move, scale and
    /// rotate it like any other object. (Legacy v1 format)
    #[serde(default)]
    pub terrain: Terrain,
    /// Static visual props (trees, houses, rocks, ...). (Legacy v1 format)
    #[serde(default)]
    pub props: Vec<Prop>,
    /// World metrics and movement constants. (v2 format)
    #[serde(default)]
    pub world_metrics: Option<WorldMetrics>,
    /// Walkable surfaces for height-aware movement. (v2 format)
    #[serde(default)]
    pub surfaces: Vec<WalkableSurface>,
    /// Traversal objects (stairs, ramps, etc.). (v2 format)
    #[serde(default)]
    pub traversals: Vec<TraversalData>,
    /// Blocking objects that prevent movement. (v2 format)
    #[serde(default)]
    pub blockers: Vec<BlockerData>,
    /// Optional test route for validation. (v2 format)
    #[serde(default)]
    pub test_route: Vec<TestRoutePoint>,
    /// Optional test checklist for validation. (v2 format)
    #[serde(default)]
    pub test_checklist: Vec<String>,
    /// Optional mountain switchback test data. (v2 format)
    #[serde(default)]
    pub mountain_switchback_test: Option<SwitchbackTest>,
    /// Optional distant plateau test data. (v2 format)
    #[serde(default)]
    pub distant_plateau_test: Option<PlateauTest>,
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

/// The walkable ground of the map: a single large cube (unit mesh, so `scale`
/// is the cube's full size in world units). The default keeps the top face on
/// the y = 0 plane, matching the flat-ground convention of the game.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Terrain {
    /// Full spatial transform (unit cube scaled by `transform.scale`).
    pub transform: TransformData,
    /// Optional color tint (linear RGB, 0..1). Absent -> engine default.
    pub tint: Option<[f32; 3]>,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            transform: TransformData {
                translation: [0.0, -0.5, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [40.0, 1.0, 40.0],
            },
            tint: None,
        }
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
    /// Logical content id (e.g. `"tree_oak"`). Validated against the
    /// [`PlaceableRegistry`](crate::placeables::PlaceableRegistry) by the loader.
    pub kind: KindId,
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

// ==================== VERSION 2 GAMEPLAY DATA STRUCTURES ====================

/// World metrics and movement constants for gameplay validation.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct WorldMetrics {
    /// Player collision radius in world units.
    pub player_radius: f32,
    /// Player height in world units.
    pub player_height: f32,
    /// Eye/camera height above the ground in world units.
    pub eye_height: f32,
    /// Maximum step height that can be climbed without jumping.
    pub max_step_height: f32,
    /// Maximum walkable slope angle in degrees.
    pub max_walkable_slope_deg: f32,
}

impl Default for WorldMetrics {
    fn default() -> Self {
        Self {
            player_radius: 0.35,
            player_height: 1.7,
            eye_height: 1.6,
            max_step_height: 0.45,
            max_walkable_slope_deg: 45.0,
        }
    }
}

impl WorldMetrics {
    /// Validates that all metrics are positive and within reasonable ranges.
    pub fn validate(&self) -> Result<(), String> {
        if self.player_radius <= 0.0 {
            return Err("player_radius must be positive".to_string());
        }
        if self.player_height <= 0.0 {
            return Err("player_height must be positive".to_string());
        }
        if self.eye_height <= 0.0 || self.eye_height > self.player_height {
            return Err("eye_height must be positive and not exceed player_height".to_string());
        }
        if self.max_step_height < 0.0 {
            return Err("max_step_height must be non-negative".to_string());
        }
        if self.max_walkable_slope_deg <= 0.0 || self.max_walkable_slope_deg > 90.0 {
            return Err("max_walkable_slope_deg must be between 0 and 90 degrees".to_string());
        }
        Ok(())
    }
}

/// Walkable surface for height-aware movement and ground queries.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WalkableSurface {
    /// Stable unique id within the map.
    pub id: String,
    /// Kind of surface (flat, mesh, etc.).
    pub kind: SurfaceKind,
    /// Object name reference (for mesh-based surfaces).
    #[serde(default)]
    pub object: Option<String>,
    /// Optional surface bounds (for flat surfaces).
    #[serde(default)]
    pub bounds: Option<SurfaceBounds>,
    /// Optional constant height (for flat_mesh surfaces).
    #[serde(default)]
    pub height: Option<f32>,
    /// Optional min/max height range (for validation).
    #[serde(default)]
    pub min_height: Option<f32>,
    #[serde(default)]
    pub max_height: Option<f32>,
    /// Optional grid size for spatial queries.
    #[serde(default)]
    pub grid_size: Option<u32>,
    /// Physical size of the surface.
    #[serde(default)]
    pub size: Option<f32>,
    /// Optional purpose description.
    #[serde(default)]
    pub purpose: Option<String>,
    /// Optional heightfield data for mesh surfaces.
    #[serde(default)]
    pub heightfield: Option<HeightfieldData>,
    /// Triangulated mesh data for precise point-in-triangle ground queries.
    ///
    /// This is the preferred representation for ramps, curved paths, and any
    /// surface whose walkable footprint is not a simple rectangle. When
    /// present, the query performs an exact triangle test instead of relying
    /// on the 2D bounds broad-phase alone.
    #[serde(default)]
    pub walkable_mesh: Option<WalkableMeshData>,
    /// Optional layer / connectivity group this surface belongs to.
    #[serde(default)]
    pub layer: Option<String>,
    /// Optional per-surface maximum walkable slope in degrees. Overrides the
    /// global `max_walkable_slope_deg` when set.
    #[serde(default)]
    pub max_slope_deg: Option<f32>,
}

/// Triangulated walkable mesh data exported from Blender.
///
/// Vertices are in world space (Y-up after glTF conversion). Indices reference
/// into the `vertices` array in groups of three (one triangle per triplet).
/// The winding must be counter-clockwise when viewed from above so that the
/// computed face normal points up.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WalkableMeshData {
    /// Flat list of vertex positions `[x, y, z]`.
    pub vertices: Vec<[f32; 3]>,
    /// Triangle indices into `vertices`. Length must be a multiple of 3.
    pub indices: Vec<u32>,
}

/// Kind of walkable surface.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum SurfaceKind {
    /// Flat surface with constant height.
    #[serde(alias = "flat")]
    Flat,
    /// Mesh-based surface with variable height.
    #[serde(alias = "mesh")]
    Mesh,
    /// Flat mesh (constant height but visual mesh).
    #[serde(alias = "flat_mesh")]
    FlatMesh,
}

/// 2D bounds for surfaces in the X/Z plane.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct SurfaceBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl SurfaceBounds {
    pub fn contains(&self, x: f32, z: f32) -> bool {
        x >= self.min_x && x <= self.max_x && z >= self.min_z && z <= self.max_z
    }
}

/// Heightfield data for mesh-based walkable surfaces.
///
/// Provides a compact representation of terrain height data that can be
/// serialized to JSON and used for efficient height queries.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HeightfieldData {
    /// Grid resolution (number of cells in each direction).
    pub resolution: u32,
    /// World space bounds of the heightfield.
    pub bounds: SurfaceBounds,
    /// Height values in row-major order (z varies fastest).
    /// Length must be (resolution + 1) * (resolution + 1).
    pub heights: Vec<f32>,
}

impl HeightfieldData {
    /// Creates a new heightfield with the given data.
    pub fn new(resolution: u32, bounds: SurfaceBounds, heights: Vec<f32>) -> Self {
        Self {
            resolution,
            bounds,
            heights,
        }
    }

    /// Validates that the heightfield data is consistent.
    pub fn validate(&self) -> Result<(), String> {
        let expected_size = (self.resolution + 1) * (self.resolution + 1);
        if self.heights.len() != expected_size as usize {
            return Err(format!(
                "Heightfield has {} heights but expected {} for resolution {}",
                self.heights.len(),
                expected_size,
                self.resolution
            ));
        }

        if self.bounds.min_x >= self.bounds.max_x {
            return Err("Heightfield bounds.min_x must be less than max_x".to_string());
        }

        if self.bounds.min_z >= self.bounds.max_z {
            return Err("Heightfield bounds.min_z must be less than max_z".to_string());
        }

        Ok(())
    }

    /// Samples height at the given world position using bilinear interpolation.
    /// Returns None if the position is outside the heightfield bounds.
    pub fn sample_height(&self, x: f32, z: f32) -> Option<f32> {
        if !self.bounds.contains(x, z) {
            return None;
        }

        // Convert world coordinates to grid coordinates
        let grid_size = self.resolution as f32;
        let cell_size_x = (self.bounds.max_x - self.bounds.min_x) / grid_size;
        let cell_size_z = (self.bounds.max_z - self.bounds.min_z) / grid_size;

        let local_x = (x - self.bounds.min_x) / cell_size_x;
        let local_z = (z - self.bounds.min_z) / cell_size_z;

        let x0 = local_x.floor() as i32;
        let x1 = x0 + 1;
        let z0 = local_z.floor() as i32;
        let z1 = z0 + 1;

        let fx = local_x - x0 as f32;
        let fz = local_z - z0 as f32;

        // Clamp to grid bounds
        let x0 = x0.max(0).min(self.resolution as i32) as usize;
        let x1 = x1.max(0).min(self.resolution as i32) as usize;
        let z0 = z0.max(0).min(self.resolution as i32) as usize;
        let z1 = z1.max(0).min(self.resolution as i32) as usize;

        // Sample the four corners of the cell
        let stride = (self.resolution + 1) as usize;
        // Heightfields are stored with Z varying fastest: index = x * stride + z.
        // Using the conventional row-major index here transposed the terrain,
        // making movement sample a height from a different location on maps
        // whose slopes differ along X and Z.
        let h00 = self.heights[x0 * stride + z0];
        let h10 = self.heights[x1 * stride + z0];
        let h01 = self.heights[x0 * stride + z1];
        let h11 = self.heights[x1 * stride + z1];

        // Bilinear interpolation
        let h0 = h00 * (1.0 - fx) + h10 * fx;
        let h1 = h01 * (1.0 - fx) + h11 * fx;
        let height = h0 * (1.0 - fz) + h1 * fz;

        Some(height)
    }

    /// Estimates the local surface normal from neighboring height samples.
    ///
    /// Heightfields are compact gameplay data, not render meshes, so normals are
    /// reconstructed from central differences. This lets movement reject slopes
    /// above the map's walkable limit without loading the GLB on the server.
    /// Boundary samples use clamped one-sided differences.
    ///
    /// # Example
    /// ```rust
    /// use bevymmo_shared::world::{HeightfieldData, SurfaceBounds};
    ///
    /// let heightfield = HeightfieldData::new(
    ///     1,
    ///     SurfaceBounds { min_x: 0.0, max_x: 1.0, min_z: 0.0, max_z: 1.0 },
    ///     vec![0.0, 0.0, 0.0, 0.0],
    /// );
    /// assert_eq!(heightfield.sample_normal(0.5, 0.5), Some([0.0, 1.0, 0.0]));
    /// ```
    pub fn sample_normal(&self, x: f32, z: f32) -> Option<[f32; 3]> {
        if !self.bounds.contains(x, z) {
            return None;
        }
        if self.resolution == 0 {
            return Some([0.0, 1.0, 0.0]);
        }

        let grid_size = self.resolution as f32;
        let cell_size_x = (self.bounds.max_x - self.bounds.min_x) / grid_size;
        let cell_size_z = (self.bounds.max_z - self.bounds.min_z) / grid_size;

        let left_x = (x - cell_size_x).max(self.bounds.min_x);
        let right_x = (x + cell_size_x).min(self.bounds.max_x);
        let down_z = (z - cell_size_z).max(self.bounds.min_z);
        let up_z = (z + cell_size_z).min(self.bounds.max_z);

        let Some(left_height) = self.sample_height(left_x, z) else {
            return None;
        };
        let Some(right_height) = self.sample_height(right_x, z) else {
            return None;
        };
        let Some(down_height) = self.sample_height(x, down_z) else {
            return None;
        };
        let Some(up_height) = self.sample_height(x, up_z) else {
            return None;
        };

        let dx = right_x - left_x;
        let dz = up_z - down_z;
        if dx.abs() < f32::EPSILON || dz.abs() < f32::EPSILON {
            return Some([0.0, 1.0, 0.0]);
        }

        let dhdx = (right_height - left_height) / dx;
        let dhdz = (up_height - down_height) / dz;
        let normal = [-dhdx, 1.0, -dhdz];
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if length < f32::EPSILON {
            return Some([0.0, 1.0, 0.0]);
        }

        Some([normal[0] / length, normal[1] / length, normal[2] / length])
    }
}

/// Traversal data for stairs, ramps, and other movement aids.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TraversalData {
    /// Stable unique id within the map.
    pub id: String,
    /// Kind of traversal.
    pub kind: TraversalKind,
    /// Start position in world space.
    pub start: [f32; 3],
    /// End position in world space.
    pub end: [f32; 3],
    /// Width of the traversal path.
    pub width: f32,
    /// Optional reference to start surface.
    pub start_surface: Option<String>,
    /// Optional reference to end surface.
    pub end_surface: Option<String>,
}

/// Kind of traversal object.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum TraversalKind {
    #[serde(alias = "ramp")]
    Ramp,
    #[serde(alias = "stairs")]
    Stairs,
}

impl TraversalData {
    /// Validates traversal data for basic correctness.
    pub fn validate(&self) -> Result<(), String> {
        if self.width <= 0.0 {
            return Err(format!("Traversal '{}' has non-positive width", self.id));
        }

        let horizontal_length =
            ((self.end[0] - self.start[0]).powi(2) + (self.end[2] - self.start[2]).powi(2)).sqrt();
        if horizontal_length < 0.01 {
            return Err(format!(
                "Traversal '{}' has negligible horizontal length",
                self.id
            ));
        }

        Ok(())
    }
}

/// Blocking object that prevents movement.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlockerData {
    /// Stable unique id within the map.
    pub id: String,
    /// Kind of blocker.
    pub kind: BlockerKind,
    /// Object name reference.
    #[serde(default)]
    pub object: Option<String>,
    /// World-space transform of the blocker volume.
    ///
    /// When present the runtime uses this instead of looking up the GLB node,
    /// so the server (headless) can resolve collisions without visual assets.
    #[serde(default)]
    pub transform: Option<TransformData>,
    /// Collision shape (box half-extents, cylinder radius/height, …).
    #[serde(default)]
    pub shape: Option<CollisionShape>,
    /// Whether this blocker prevents movement. Defaults to `true`.
    #[serde(default = "default_blocks_movement")]
    pub blocks_movement: bool,
}

/// Default value for [`BlockerData::blocks_movement`].
fn default_blocks_movement() -> bool {
    true
}

/// Kind of blocking object.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum BlockerKind {
    /// Axis-aligned box blocker.
    #[serde(alias = "box")]
    Box,
    /// Cylinder blocker.
    #[serde(alias = "cylinder")]
    Cylinder,
}

/// Test route point for validation.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct TestRoutePoint {
    pub x: f32,
    pub z: f32,
    /// Optional in older world manifests; zero means base elevation.
    #[serde(default)]
    pub height: f32,
}

/// Switchback test data for mountain climbing validation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SwitchbackTest {
    pub start: TestRoutePoint,
    pub summit: TestRoutePoint,
    pub route: Vec<TestRoutePoint>,
    pub expected: String,
}

/// Plateau test data for distant plateau validation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlateauTest {
    pub start: TestRoutePoint,
    pub plateau: TestRoutePoint,
    pub route: Vec<TestRoutePoint>,
    pub expected: String,
}

// ==================== MANIFEST HELPER METHODS ====================

impl MapManifest {
    /// Returns true if this manifest uses version 2 format with gameplay data.
    pub fn is_v2(&self) -> bool {
        self.version >= 2
    }

    /// Returns true if this manifest uses version 1 format (legacy).
    pub fn is_v1(&self) -> bool {
        self.version == 1
    }

    /// Gets the world metrics, using defaults for v1 manifests.
    pub fn get_world_metrics(&self) -> WorldMetrics {
        self.world_metrics.unwrap_or_default()
    }

    /// Validates the manifest for basic correctness.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != CURRENT_VERSION && self.version != LEGACY_VERSION_1 {
            return Err(format!("Unknown manifest version: {}", self.version));
        }

        if self.map_id.is_empty() {
            return Err("map_id must not be empty".to_string());
        }

        if self.bounds.min_x >= self.bounds.max_x {
            return Err("bounds.min_x must be less than bounds.max_x".to_string());
        }

        if self.bounds.min_z >= self.bounds.max_z {
            return Err("bounds.min_z must be less than bounds.max_z".to_string());
        }

        // Validate world metrics if present
        if let Some(metrics) = self.world_metrics {
            metrics.validate()?;
        }

        // Validate traversals
        for traversal in &self.traversals {
            traversal.validate()?;
        }

        // Validate surface IDs are unique
        let mut surface_ids = std::collections::HashSet::new();
        for surface in &self.surfaces {
            if !surface_ids.insert(&surface.id) {
                return Err(format!("Duplicate surface id: {}", surface.id));
            }
        }

        // Validate blocker IDs are unique
        let mut blocker_ids = std::collections::HashSet::new();
        for blocker in &self.blockers {
            if !blocker_ids.insert(&blocker.id) {
                return Err(format!("Duplicate blocker id: {}", blocker.id));
            }
        }

        Ok(())
    }
}
