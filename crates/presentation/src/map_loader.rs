//! Map loader/saver: supports both `.ron` (legacy) and `.glb` (Blender export).
//!
//! The `.glb` format stores the same logical data (`MapManifest`) inside a
//! glTF 2.0 binary file. Each prop node carries `extras` JSON with the
//! `bevymmo` key containing `kind`, collision data, etc.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use ron::ser::PrettyConfig;
use serde::Deserialize;
use serde_json;
use thiserror::Error;

use bevymmo_gameplay::placeables::{KindId, PlaceableRegistry};
use bevymmo_world::{
    validate_id, CollisionShape, HeightfieldData, MapBounds, MapManifest, Prop, SurfaceBounds,
    SurfaceKind, Terrain, TransformData, WalkableMeshData, WalkableSurface, CURRENT_VERSION,
};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum MapLoadError {
    #[error("failed to read map file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse RON map file {path}: {source}")]
    Ron {
        path: String,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("failed to parse JSON map file {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to parse GLB map file {path}: {source}")]
    Gltf {
        path: String,
        #[source]
        source: gltf::Error,
    },
    #[error("failed to serialize map file {path}: {message}")]
    Serialize { path: String, message: String },
    #[error("manifest validation failed: {0:?}")]
    Validation(Vec<ValidationIssue>),
    #[error("node '{node_name}' is missing required bevymmo extras")]
    MissingExtras { node_name: String },
    #[error("invalid JSON in bevymmo extras on node '{node_name}': {message}")]
    InvalidExtras { node_name: String, message: String },
}

// ---------------------------------------------------------------------------
// Extras DTOs — what lives inside glTF node `extras.bevymmo`
// ---------------------------------------------------------------------------

/// Top-level metadata stored in the special `__bevymmo_map_meta` node.
#[derive(Deserialize)]
struct MapMetaExtras {
    #[serde(rename = "_meta")]
    _meta: bool,
    map_id: String,
    display_name: String,
    bounds: BoundsExtras,
}

#[derive(Deserialize)]
struct BoundsExtras {
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
}

/// Per-prop data stored in each prop node's `extras.bevymmo` (or flat
/// with `bevymmo_` prefix for Blender native extras export).
#[derive(Deserialize)]
struct PropExtras {
    #[serde(alias = "bevymmo_kind")]
    kind: String,
    #[serde(alias = "bevymmo_id", default = "default_id_from_node")]
    id: String,
    #[serde(default)]
    collision: Option<CollisionExtras>,
    // Flat-format fields (Blender native extras export)
    #[serde(alias = "bevymmo_collision", default)]
    collision_type_flat: Option<String>,
    #[serde(alias = "bevymmo_radius", default)]
    radius_flat: Option<serde_json::Value>,
    #[serde(alias = "bevymmo_height", default)]
    height_flat: Option<serde_json::Value>,
    #[serde(alias = "bevymmo_half_extents", default)]
    half_extents_flat: Option<serde_json::Value>,
    #[serde(alias = "bevymmo_blocks_move", default)]
    blocks_movement: bool,
    #[serde(alias = "bevymmo_tint", default)]
    tint: Option<[f32; 3]>,
}

fn default_id_from_node() -> String {
    "auto".to_string()
}

impl PropExtras {
    /// Extracts the collision shape from either nested (`collision: {...}`)
    /// or flat (`bevymmo_collision: "cylinder"`, `bevymmo_radius: ...`) format.
    fn to_collision_shape(&self) -> Option<CollisionShape> {
        // Prefer nested format
        if let Some(ref nested) = self.collision {
            // Clone the actual CollisionExtras struct, not the reference
            let nested_copy = CollisionExtras {
                type_: nested.type_.clone(),
                radius: nested.radius.clone(),
                height: nested.height.clone(),
                half_extents: nested.half_extents.clone(),
            };
            return CollisionShape::try_from(nested_copy).ok();
        }

        // Fall back to flat format (Blender native extras)
        if let Some(ref type_str) = self.collision_type_flat {
            match type_str.as_str() {
                "cylinder" => Some(CollisionShape::Cylinder {
                    radius: json_to_f32(&self.radius_flat).unwrap_or(0.5),
                    height: json_to_f32(&self.height_flat).unwrap_or(2.0),
                }),
                "box" => Some(CollisionShape::Box {
                    half_extents: json_to_vec3(&self.half_extents_flat).unwrap_or([0.5, 0.5, 0.5]),
                }),
                "sphere" => Some(CollisionShape::Sphere {
                    radius: json_to_f32(&self.radius_flat).unwrap_or(0.5),
                }),
                "none" | "" => None,
                _ => None,
            }
        } else {
            None
        }
    }
}

/// Collision shape definition — fields use `serde_json::Value` because Blender's
/// glTF extras export writes all custom properties as **strings**, not numbers.
#[derive(Deserialize)]
struct CollisionExtras {
    #[serde(rename = "type")]
    type_: String,
    /// Radius — may be an f32 or a string like `"0.5"`.
    radius: Option<serde_json::Value>,
    /// Height — may be an f32 or a string.
    height: Option<serde_json::Value>,
    /// Half extents — may be [f32;3] or a comma-separated string like `"1,1,1"`.
    half_extents: Option<serde_json::Value>,
}

/// Extracts an f32 from a JSON value that might be a number or a string.
fn json_to_f32(val: &Option<serde_json::Value>) -> Option<f32> {
    val.as_ref()?
        .as_f64()
        .map(|v| v as f32)
        .or_else(|| val.as_ref()?.as_str()?.parse::<f32>().ok())
}

/// Extracts [f32;3] from a JSON value that might be an array or comma-separated string.
fn json_to_vec3(val: &Option<serde_json::Value>) -> Option<[f32; 3]> {
    // Try array of numbers first
    if let Some(arr) = val.as_ref()?.as_array() {
        if arr.len() >= 3 {
            let v: Vec<f32> = arr
                .iter()
                .take(3)
                .filter_map(|e| e.as_f64().map(|x| x as f32))
                .collect();
            if v.len() == 3 {
                return Some([v[0], v[1], v[2]]);
            }
        }
    }

    // Try comma-separated string (Blender's native format for custom properties)
    if let Some(s) = val.as_ref()?.as_str() {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() >= 3 {
            let v: Vec<f32> = parts
                .iter()
                .take(3)
                .filter_map(|x| x.trim().parse().ok())
                .collect();
            if v.len() == 3 {
                return Some([v[0], v[1], v[2]]);
            }
        }
    }

    None
}

impl TryFrom<CollisionExtras> for CollisionShape {
    type Error = String;

    fn try_from(value: CollisionExtras) -> Result<Self, Self::Error> {
        match value.type_.as_str() {
            "cylinder" => Ok(CollisionShape::Cylinder {
                radius: json_to_f32(&value.radius)
                    .ok_or_else(|| "cylinder missing 'radius'".to_string())?,
                height: json_to_f32(&value.height)
                    .ok_or_else(|| "cylinder missing 'height'".to_string())?,
            }),
            "box" => Ok(CollisionShape::Box {
                half_extents: json_to_vec3(&value.half_extents)
                    .ok_or_else(|| "box missing 'half_extents'".to_string())?,
            }),
            "sphere" => Ok(CollisionShape::Sphere {
                radius: json_to_f32(&value.radius)
                    .ok_or_else(|| "sphere missing 'radius'".to_string())?,
            }),
            other => Err(format!("unknown collision type '{other}'")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationIssue {
    pub message: String,
}

impl ValidationIssue {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Loads a map from a `.ron` file and runs validation. Validation errors
/// become `MapLoadError::Validation` so callers can surface every issue at once.
pub fn load_map<P: AsRef<Path>>(path: P) -> Result<MapManifest, MapLoadError> {
    let path_ref = path.as_ref();
    let path_display = path_ref.display().to_string();

    let source = fs::read_to_string(path_ref).map_err(|source| MapLoadError::Io {
        path: path_display.clone(),
        source,
    })?;

    let manifest: MapManifest = ron::from_str(&source).map_err(|source| MapLoadError::Ron {
        path: path_display,
        source,
    })?;

    let issues = validate_structure(&manifest);
    if !issues.is_empty() {
        return Err(MapLoadError::Validation(issues));
    }

    Ok(manifest)
}

/// Loads a map from a `.world.json` sidecar file and runs validation.
///
/// Version 2 maps use `.world.json` sidecar files alongside `.glb` files.
/// The JSON file contains gameplay-specific data:
/// - Walkable surfaces for height-aware movement
/// - Traversal data (stairs, ramps, etc.)
/// - Blocker data for movement prevention
/// - World metrics and validation data
///
/// Validation errors become `MapLoadError::Validation` so callers can
/// surface every issue at once.
pub fn load_world_json<P: AsRef<Path>>(path: P) -> Result<MapManifest, MapLoadError> {
    let path_ref = path.as_ref();
    let path_display = path_ref.display().to_string();

    let source = fs::read_to_string(path_ref).map_err(|source| MapLoadError::Io {
        path: path_display.clone(),
        source,
    })?;

    let manifest: MapManifest =
        serde_json::from_str(&source).map_err(|source| MapLoadError::Json {
            path: path_display.clone(),
            source,
        })?;

    let issues = validate_structure(&manifest);
    if !issues.is_empty() {
        return Err(MapLoadError::Validation(issues));
    }

    warn_on_map_id_mismatch(path_ref, &manifest);

    Ok(manifest)
}

/// Warns when a sidecar's `map_id` does not match its own filename.
///
/// Nothing else ties the two together, and they are used for different things:
/// the loader picks the sidecar by filename, while the client renders
/// `maps/<manifest.map_id>.glb`. A sidecar copied from another map keeps the
/// original `map_id`, and the result is a map whose collision comes from one
/// file and whose visuals come from a different one — the player then appears
/// to stand under (or float above) terrain that is not the terrain being
/// walked on. Both files load fine, so nothing else surfaces the mistake.
fn warn_on_map_id_mismatch(path: &Path, manifest: &MapManifest) {
    // `map_02.world.json` yields the file stem `map_02.world`, so strip the
    // inner extension too before comparing.
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    let expected = stem.strip_suffix(".world").unwrap_or(stem);
    if expected != manifest.map_id {
        bevy::log::warn!(
            "Map sidecar {} declares map_id {:?}: gameplay data comes from this file, \
             but visuals load from maps/{}.glb. Rename the file or fix map_id.",
            path.display(),
            manifest.map_id,
            manifest.map_id
        );
    }
}

/// Checks if a `.world.json` sidecar file exists for the given map ID.
///
/// Given a map path like `/path/to/maps/my_map.glb`, this function checks
/// if `/path/to/maps/my_map.world.json` exists.
pub fn has_world_json_sidecar<P: AsRef<Path>>(map_path: P) -> bool {
    let map_path = map_path.as_ref();

    // Build the expected sidecar path by replacing the extension
    let sidecar_path = map_path.with_extension("world.json");

    sidecar_path.exists() && sidecar_path.is_file()
}

// ---------------------------------------------------------------------------
// GLB loader — reads a glTF 2.0 binary exported from Blender
// ---------------------------------------------------------------------------

/// Prefix used by Blender-authored walkable surface objects.
///
/// The level designer names any mesh that the player should be able to walk
/// on with this prefix (e.g. `WALKABLE_main_floor`, `WALKABLE_ramp_01`). The
/// GLB loader picks those up automatically and registers them as walkable
/// surfaces even when no `.world.json` sidecar is present.
const WALKABLE_NODE_PREFIX: &str = "WALKABLE_";

/// `bevymmo_kind` value that marks a node as a walkable surface. Used by
/// the Blender add-on when it wants to be explicit instead of relying on the
/// `WALKABLE_` naming convention.
const WALKABLE_KIND_TAG: &str = "walkable_surface";

/// Default heightfield sampling resolution baked into auto-extracted
/// surfaces when no `.world.json` sidecar overrides it.
///
/// A 32x32 grid keeps the per-cell footprint ≈1m on a typical map and is
/// the same default used by the existing `.world.json` fixtures.
const DEFAULT_HEIGHTFIELD_RESOLUTION: u32 = 32;

/// Returns `true` when a glTF node represents a walkable surface.
///
/// Detection is name-based (`WALKABLE_*` prefix) or extras-based
/// (`bevymmo_kind == "walkable_surface"`). The extras check is permissive:
/// any malformed JSON is treated as "not a surface".
fn is_walkable_node(node: &gltf::Node<'_>) -> bool {
    if node
        .name()
        .is_some_and(|name| name.starts_with(WALKABLE_NODE_PREFIX))
    {
        return true;
    }
    let Some(extras_raw) = node.extras() else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(extras_raw.get()) else {
        return false;
    };
    let kind = value
        .as_object()
        .and_then(|object| {
            object
                .get("bevymmo")
                .and_then(|b| b.get("kind"))
                .or_else(|| object.get("bevymmo_kind"))
        })
        .and_then(|k| k.as_str())
        .unwrap_or("");
    kind == WALKABLE_KIND_TAG
}

/// Extracts a single mesh primitive's vertex positions and triangle indices
/// from a glTF node, transformed into world space.
///
/// Returns `None` when the node has no mesh, the primitive has no POSITION
/// accessor, or the buffer data cannot be resolved from the embedded GLB
/// blob. Non-indexed geometry is converted to a sequential index list.
fn extract_node_mesh_world_space(
    node: &gltf::Node<'_>,
    blob: Option<&[u8]>,
) -> Option<WalkableMeshData> {
    let mesh = node.mesh()?;
    let primitive = mesh.primitives().next()?;
    let reader = primitive.reader(move |buffer: gltf::Buffer<'_>| match buffer.source() {
        gltf::buffer::Source::Bin => blob,
        gltf::buffer::Source::Uri(_) => None,
    });

    let positions = reader.read_positions()?;

    // glTF stores node transforms as either a 4x4 matrix or a decomposed
    // (T, R, S) triple. `Transform::matrix()` normalises both into a
    // column-major 4x4 we can apply per-vertex.
    let matrix = node.transform().matrix();

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    for position in positions {
        let [x, y, z] = position;
        // Column-major multiplication: v' = M * v (homogeneous, w=1).
        let tx = matrix[0][0] * x + matrix[1][0] * y + matrix[2][0] * z + matrix[3][0];
        let ty = matrix[0][1] * x + matrix[1][1] * y + matrix[2][1] * z + matrix[3][1];
        let tz = matrix[0][2] * x + matrix[1][2] * y + matrix[2][2] * z + matrix[3][2];
        vertices.push([tx, ty, tz]);
    }

    // Index list: fall back to sequential indices for non-indexed geometry
    // so the downstream triangle resolver has a uniform contract.
    let indices: Vec<u32> = match reader.read_indices() {
        Some(reader_indices) => reader_indices.into_u32().collect(),
        None => (0..vertices.len() as u32).collect(),
    };

    if vertices.is_empty() || indices.len() < 3 {
        return None;
    }

    Some(WalkableMeshData { vertices, indices })
}

/// Builds a [`WalkableSurface`] from a glTF walkable node.
///
/// The surface stores the mesh vertices in world space plus an optional
/// coarse heightfield (32x32 by default) covering the mesh's XZ bounds. The
/// heightfield is what `SurfaceQuery::ground_at` falls back to for fast
/// broad-phase lookups when no explicit `.world.json` is provided.
fn build_surface_from_node(node: &gltf::Node<'_>, blob: Option<&[u8]>) -> Option<WalkableSurface> {
    let mesh_data = extract_node_mesh_world_space(node, blob)?;
    let bounds = compute_mesh_bounds(&mesh_data.vertices);
    let heightfield = build_coarse_heightfield(&mesh_data, bounds, DEFAULT_HEIGHTFIELD_RESOLUTION);
    let object_name = node.name().map(|name| name.to_string());

    Some(WalkableSurface {
        id: object_name
            .clone()
            .unwrap_or_else(|| format!("surface_{}", node.index())),
        kind: SurfaceKind::Mesh,
        object: object_name,
        bounds: Some(bounds),
        height: None,
        min_height: None,
        max_height: None,
        grid_size: None,
        size: None,
        purpose: None,
        heightfield,
        walkable_mesh: Some(mesh_data),
        layer: None,
        max_slope_deg: None,
    })
}

/// Computes the XZ footprint of a triangle mesh in world space.
fn compute_mesh_bounds(vertices: &[[f32; 3]]) -> SurfaceBounds {
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;

    for [x, _y, z] in vertices {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_z = min_z.min(*z);
        max_z = max_z.max(*z);
    }

    SurfaceBounds {
        min_x,
        max_x,
        min_z,
        max_z,
    }
}

/// Samples a regular `resolution`x`resolution` grid over the mesh's XZ
/// bounds and stores the highest vertex Y per cell.
///
/// This is intentionally a coarse "max height" approximation — precise
/// ground resolution happens at query time via the triangle-mesh resolver.
/// The heightfield exists so broad-phase containment checks and the
/// default movement code can produce sensible heights for surfaces that
/// ship without a hand-authored `.world.json`.
fn build_coarse_heightfield(
    mesh: &WalkableMeshData,
    bounds: SurfaceBounds,
    resolution: u32,
) -> Option<HeightfieldData> {
    if mesh.vertices.is_empty() {
        return None;
    }

    let dx = ((bounds.max_x - bounds.min_x) / resolution as f32).max(1e-6);
    let dz = ((bounds.max_z - bounds.min_z) / resolution as f32).max(1e-6);

    // (resolution + 1)² samples, matching HeightfieldData's contract.
    let side = (resolution + 1) as usize;
    let mut heights = vec![f32::NEG_INFINITY; side * side];

    for [x, y, z] in &mesh.vertices {
        let gx = (((x - bounds.min_x) / dx).round() as isize).clamp(0, resolution as isize);
        let gz = (((z - bounds.min_z) / dz).round() as isize).clamp(0, resolution as isize);
        let idx = gx as usize + gz as usize * side;
        if *y > heights[idx] {
            heights[idx] = *y;
        }
    }

    // Replace untouched cells with the mesh's min Y so queries return a
    // finite value instead of -∞.
    let min_y = mesh
        .vertices
        .iter()
        .map(|v| v[1])
        .fold(f32::INFINITY, f32::min);
    for h in heights.iter_mut() {
        if !h.is_finite() {
            *h = min_y;
        }
    }

    Some(HeightfieldData::new(resolution, bounds, heights))
}

/// Walks all glTF root nodes and returns every node flagged as a walkable
/// surface (see [`is_walkable_node`]).
fn extract_walkable_surfaces(
    document: &gltf::Document,
    blob: Option<&[u8]>,
) -> Vec<WalkableSurface> {
    document
        .nodes()
        .filter(is_walkable_node)
        .filter_map(|node| build_surface_from_node(&node, blob))
        .collect()
}

/// Loads a map from a `.glb` file.
///
/// The GLB must follow the BevyMMO convention:
/// - One node named `__bevymmo_map_meta` carries map-level metadata in its
///   `extras.bevymmo` JSON (`map_id`, `display_name`, `bounds`).
/// - Every other node with an `extras.bevymmo.kind` field is treated as a
///   [`Prop`]. The node's transform provides position/rotation/scale; the
///   extras carry logical data (`collision`, `blocks_movement`, `tint`).
///
/// This function uses only the **data** portion of the `gltf` crate (no GPU,
/// no image decoding), so it compiles on the headless server.
pub fn load_map_from_glb<P: AsRef<Path>>(path: P) -> Result<MapManifest, MapLoadError> {
    let path_ref = path.as_ref();
    let path_display = path_ref.display().to_string();

    let bytes = fs::read(path_ref).map_err(|source| MapLoadError::Io {
        path: path_display.clone(),
        source,
    })?;

    let gltf = gltf::Gltf::from_slice(&bytes).map_err(|source| MapLoadError::Gltf {
        path: path_display.clone(),
        source,
    })?;

    let root = &gltf.document;

    // --- Find the meta node and collect props ---
    let mut manifest_meta: Option<MapMetaExtras> = None;
    let mut props = Vec::new();

    for node in root.nodes() {
        let name = node.name().unwrap_or("<unnamed>");
        let Some(extras_raw) = node.extras() else {
            continue;
        };

        // Parse extras JSON string into a generic json::Value
        let extras_value: serde_json::Value =
            serde_json::from_str(extras_raw.get()).map_err(|e| MapLoadError::InvalidExtras {
                node_name: name.to_string(),
                message: e.to_string(),
            })?;

        // Support two formats:
        // 1. Nested: { "bevymmo": { "kind": "...", ... } }  — explicit export script
        // 2. Flat:   { "bevymmo_kind": "...", ... }          — Blender native extras
        let bm = extras_value
            .as_object()
            .and_then(|o| o.get("bevymmo"))
            .cloned();

        // Special node: __bevymmo_map_meta carries map metadata
        if name == "__bevymmo_map_meta" || name == "__bevymmo_map_meta__" {
            if let Some(ref nested) = bm {
                // Nested format
                let meta: MapMetaExtras = serde_json::from_value(nested.clone()).map_err(|e| {
                    MapLoadError::InvalidExtras {
                        node_name: name.to_string(),
                        message: format!("invalid map meta: {e}"),
                    }
                })?;
                manifest_meta = Some(meta);
            } else {
                // Flat format — read directly from extras root
                let obj = extras_value
                    .as_object()
                    .ok_or(MapLoadError::InvalidExtras {
                        node_name: name.to_string(),
                        message: "map meta extras must be a JSON object".to_string(),
                    })?;
                let get_str = |key: &str| -> Option<String> {
                    obj.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
                };
                let get_f32 = |key: &str| -> Option<f32> {
                    obj.get(key).and_then(|v| v.as_f64()).map(|f| f as f32)
                };
                manifest_meta = Some(MapMetaExtras {
                    _meta: true,
                    map_id: get_str("bevymmo_map_id").unwrap_or_else(|| "untitled".to_string()),
                    display_name: get_str("bevymmo_display_name")
                        .unwrap_or_else(|| "Untitled Map".to_string()),
                    bounds: BoundsExtras {
                        min_x: get_f32("bevymmo_min_x").unwrap_or(-20.0),
                        max_x: get_f32("bevymmo_max_x").unwrap_or(20.0),
                        min_z: get_f32("bevymmo_min_z").unwrap_or(-20.0),
                        max_z: get_f32("bevymmo_max_z").unwrap_or(20.0),
                    },
                });
            }
            continue;
        }

        // Regular prop node — must have a kind identifier
        let prop_data = if let Some(nested) = bm {
            // Nested format: extract from bevymmo object
            nested.clone()
        } else {
            // Flat format: use the whole extras as the data source
            // But only if it has a kind field
            if !extras_value
                .as_object()
                .map(|o| o.contains_key("bevymmo_kind"))
                .unwrap_or(false)
            {
                continue;
            }
            extras_value.clone()
        };

        let prop_extras: PropExtras =
            serde_json::from_value(prop_data.clone()).map_err(|e| MapLoadError::InvalidExtras {
                node_name: name.to_string(),
                message: format!("invalid prop extras: {e}"),
            })?;

        // Extract transform from the glTF node (column-major 4x4 matrix)
        let transform = node.transform();
        let (translation, rotation, scale) = transform.decomposed();

        // Convert quaternion → Euler YXZ degrees
        let rotation_deg = quat_to_euler_yxz(rotation);

        // Resolve collision shape (handles both nested and flat format)
        let collision = prop_extras.to_collision_shape();

        // Auto-generate id if still "auto"
        let prop_id = if prop_extras.id == "auto" || prop_extras.id.is_empty() {
            format!("prop_{:03}", props.len() + 1)
        } else {
            prop_extras.id
        };

        props.push(Prop {
            id: prop_id,
            kind: KindId::new(prop_extras.kind),
            transform: TransformData {
                translation: [translation[0], translation[1], translation[2]],
                rotation_deg,
                scale: [scale[0], scale[1], scale[2]],
            },
            tint: prop_extras.tint,
            collision,
            blocks_movement: prop_extras.blocks_movement,
        });
    }

    // Build the manifest
    let meta = manifest_meta.ok_or_else(|| {
        MapLoadError::Validation(vec![ValidationIssue::new(
            "missing __bevymmo_map_meta node in GLB",
        )])
    })?;

    // Auto-extract walkable surfaces from WALKABLE_* nodes (or nodes tagged
    // with `bevymmo_kind = walkable_surface`). This gives the GLB-only path a
    // usable ground query without requiring a hand-authored `.world.json`
    // sidecar. When a sidecar is present, [`load_map_auto`] prefers it and
    // this code path is never reached.
    let blob = gltf.blob.as_deref();
    let surfaces = extract_walkable_surfaces(root, blob);

    let manifest = MapManifest {
        version: CURRENT_VERSION,
        map_id: meta.map_id,
        display_name: meta.display_name,
        bounds: MapBounds {
            min_x: meta.bounds.min_x,
            max_x: meta.bounds.max_x,
            min_z: meta.bounds.min_z,
            max_z: meta.bounds.max_z,
        },
        terrain: Terrain::default(), // TODO: extract from a TERRAIN node or keep default
        props,
        world_metrics: None,
        surfaces,
        traversals: vec![],
        blockers: vec![],
        test_route: vec![],
        test_checklist: vec![],
        mountain_switchback_test: None,
        distant_plateau_test: None,
    };

    let issues = validate_structure(&manifest);
    if !issues.is_empty() {
        return Err(MapLoadError::Validation(issues));
    }

    Ok(manifest)
}

/// Converts a glTF quaternion `[x, y, z, w]` to Euler angles in degrees,
/// YXZ order (yaw around Y, then pitch around X, then roll around Z).
///
/// This matches the convention used by [`TransformData::rotation_deg`] so that
/// rotations authored in Blender round-trip correctly through the GLB.
fn quat_to_euler_yxz(q: [f32; 4]) -> [f32; 3] {
    // glTF stores quaternions as [x, y, z, w]
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);

    // Clamp to avoid NaN from floating-point drift
    let sinp = (2.0 * (w * x - y * z)).clamp(-1.0, 1.0);
    let sincos_2y = (2.0 * (w * y + z * x)).clamp(-1.0, 1.0);
    let cosc_2y = (1.0 - 2.0 * (x * x + y * y)).clamp(-1.0, 1.0);
    let sinr_cosp = 2.0 * (w * z + x * y);
    let cosr_cosp = 1.0 - 2.0 * (y * y + z * z);

    let yaw = sincos_2y.atan2(cosc_2y).to_degrees(); // Y axis
    let pitch = sinp.asin().to_degrees(); // X axis
    let roll = sinr_cosp.atan2(cosr_cosp).to_degrees(); // Z axis

    [yaw, pitch, roll]
}

// ---------------------------------------------------------------------------
// Convenience: auto-detect format by extension
// ---------------------------------------------------------------------------

/// Loads a map from either a `.world.json`, `.ron` or `.glb` file.
///
/// Checks for `.world.json` sidecar files first (version 2 format),
/// then falls back to `.glb` or `.ron` files (version 1 format).
pub fn load_map_auto<P: AsRef<Path>>(path: P) -> Result<MapManifest, MapLoadError> {
    let path_ref = path.as_ref();

    // Check if a .world.json sidecar exists first (version 2 format)
    if has_world_json_sidecar(path_ref) {
        let sidecar_path = path_ref.with_extension("world.json");
        return load_world_json(sidecar_path);
    }

    // Fall back to existing behavior for version 1 format
    let ext = path_ref.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "glb" | "gltf" => load_map_from_glb(path),
        _ => load_map(path),
    }
}

/// Serializes the manifest, picking the format from the file extension.
///
/// - `.json` (including `.world.json`) -> version 2 JSON, the format
///   [`load_map_auto`] prefers and the Blender exporter produces.
/// - `.glb` / `.gltf` -> **rejected**. Those are source meshes, not manifests;
///   writing a manifest there destroys the authored geometry.
/// - anything else -> RON, the version 1 format.
///
/// `props` are sorted by `id` so diffs stay readable across edits.
pub fn save_map<P: AsRef<Path>>(path: P, manifest: &MapManifest) -> Result<(), MapLoadError> {
    let path_ref = path.as_ref();
    let path_display = path_ref.display().to_string();

    let extension = path_ref
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Refuse to overwrite a source mesh. `save_map` writes a text manifest, so
    // pointing it at a `.glb` silently replaced a binary glTF — the authored
    // geometry — with manifest text.
    if matches!(extension.as_str(), "glb" | "gltf") {
        return Err(MapLoadError::Serialize {
            path: path_display,
            message: "refusing to write a map manifest over a glTF source mesh; \
                      save to the '.world.json' sidecar instead"
                .to_string(),
        });
    }

    let mut sorted = manifest.clone();
    sorted.props.sort_by(|a, b| a.id.cmp(&b.id));

    let serialized = if extension == "json" {
        serde_json::to_string_pretty(&sorted).map_err(|source| MapLoadError::Serialize {
            path: path_display.clone(),
            message: source.to_string(),
        })?
    } else {
        let config = PrettyConfig::new();
        ron::ser::to_string_pretty(&sorted, config).map_err(|source| MapLoadError::Serialize {
            path: path_display.clone(),
            message: source.to_string(),
        })?
    };

    fs::write(path_ref, serialized).map_err(|source| MapLoadError::Io {
        path: path_display,
        source,
    })?;
    Ok(())
}

/// Structural validation: checks that do not depend on the placeable
/// catalog (version, ids, bounds, scales, prop bounds). Used by [`load_map`]
/// and as the first pass of [`validate`].
pub fn validate_structure(manifest: &MapManifest) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if manifest.version != CURRENT_VERSION && manifest.version != 1 {
        issues.push(ValidationIssue::new(format!(
            "unknown manifest version {} (expected {})",
            manifest.version, CURRENT_VERSION
        )));
    }

    if manifest.map_id.trim().is_empty() {
        issues.push(ValidationIssue::new("map_id is empty"));
    } else if !validate_id(&manifest.map_id) {
        issues.push(ValidationIssue::new(format!(
            "map_id {:?} contains invalid characters (allowed: [A-Za-z0-9_-])",
            manifest.map_id
        )));
    }

    if manifest.display_name.trim().is_empty() {
        issues.push(ValidationIssue::new("display_name is empty"));
    }

    check_bounds(&manifest.bounds, &mut issues);

    for axis in 0..3 {
        if manifest.terrain.transform.scale[axis] <= 0.0 {
            issues.push(ValidationIssue::new(format!(
                "terrain has non-positive scale on axis {axis} ({})",
                manifest.terrain.transform.scale[axis]
            )));
        }
    }

    let mut seen_ids: HashSet<&str> = HashSet::new();
    for prop in &manifest.props {
        if !validate_id(&prop.id) {
            issues.push(ValidationIssue::new(format!(
                "prop id {:?} is not a valid identifier",
                prop.id
            )));
            continue;
        }
        if !seen_ids.insert(prop.id.as_str()) {
            issues.push(ValidationIssue::new(format!(
                "duplicate prop id {:?}",
                prop.id
            )));
        }
        for axis in 0..3 {
            if prop.transform.scale[axis] <= 0.0 {
                issues.push(ValidationIssue::new(format!(
                    "prop {:?} has non-positive scale on axis {} ({})",
                    prop.id, axis, prop.transform.scale[axis]
                )));
            }
        }
        if !manifest
            .bounds
            .contains(prop.transform.translation[0], prop.transform.translation[2])
        {
            issues.push(ValidationIssue::new(format!(
                "prop {:?} is outside map bounds (x={}, z={})",
                prop.id, prop.transform.translation[0], prop.transform.translation[2]
            )));
        }
    }

    issues
}

/// Returns all validation issues found in `manifest`, including kind checks
/// against `registry`. An empty result means the manifest is valid.
///
/// Combines [`validate_structure`] with a pass that flags any `prop.kind`
/// not present in the [`PlaceableRegistry`]. Callers without a registry
/// (e.g. [`load_map`]) should call [`validate_structure`] instead.
pub fn validate(manifest: &MapManifest, registry: &PlaceableRegistry) -> Vec<ValidationIssue> {
    let mut issues = validate_structure(manifest);
    for prop in &manifest.props {
        if !registry.contains(&prop.kind) {
            issues.push(ValidationIssue::new(format!(
                "prop {:?} has unknown kind {:?}",
                prop.id,
                prop.kind.as_str()
            )));
        }
    }
    issues
}

fn check_bounds(bounds: &MapBounds, issues: &mut Vec<ValidationIssue>) {
    if bounds.min_x >= bounds.max_x {
        issues.push(ValidationIssue::new(format!(
            "bounds min_x ({}) must be < max_x ({})",
            bounds.min_x, bounds.max_x
        )));
    }
    if bounds.min_z >= bounds.max_z {
        issues.push(ValidationIssue::new(format!(
            "bounds min_z ({}) must be < max_z ({})",
            bounds.min_z, bounds.max_z
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::manifest::{Prop, Terrain, TransformData};
    use crate::world::shapes::CollisionShape;

    fn empty_manifest() -> MapManifest {
        MapManifest {
            version: CURRENT_VERSION,
            map_id: "test_map".to_string(),
            display_name: "Test Map".to_string(),
            bounds: MapBounds {
                min_x: -10.0,
                max_x: 10.0,
                min_z: -10.0,
                max_z: 10.0,
            },
            terrain: Terrain::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        }
    }

    #[test]
    fn empty_manifest_is_valid() {
        assert!(validate_structure(&empty_manifest()).is_empty());
    }

    /// Regression: the editor's default save path pointed at `<map_id>.glb`,
    /// so `Ctrl+S` replaced the authored binary glTF with manifest text.
    #[test]
    fn save_map_refuses_to_overwrite_a_gltf_source_mesh() {
        let dir = std::env::temp_dir().join("bevymmo_save_map_glb_guard");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test_map.glb");
        fs::write(&path, b"glTF-binary-payload").expect("seed the source mesh");

        let error = save_map(&path, &empty_manifest()).expect_err("must refuse a .glb target");
        assert!(matches!(error, MapLoadError::Serialize { .. }));

        // The original bytes must be untouched.
        assert_eq!(
            fs::read(&path).expect("mesh still readable"),
            b"glTF-binary-payload"
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn save_map_writes_version_2_json_for_a_world_json_path() {
        let dir = std::env::temp_dir().join("bevymmo_save_map_world_json");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test_map.world.json");

        let manifest = empty_manifest();
        save_map(&path, &manifest).expect("write sidecar");

        let written = fs::read_to_string(&path).expect("read back");
        assert!(
            written.trim_start().starts_with('{'),
            "expected JSON, got: {written:.60}"
        );
        // Round trip through the loader the game actually uses.
        let reloaded = load_world_json(&path).expect("reload sidecar");
        assert_eq!(reloaded, manifest);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn save_map_still_writes_ron_for_a_ron_path() {
        let dir = std::env::temp_dir().join("bevymmo_save_map_ron");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test_map.ron");

        save_map(&path, &empty_manifest()).expect("write ron");

        let written = fs::read_to_string(&path).expect("read back");
        assert!(
            written.trim_start().starts_with('('),
            "expected RON, got: {written:.60}"
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn legacy_manifest_without_terrain_loads_with_default_terrain() {
        // Maps written before the `terrain` field existed must still load.
        let source = r#"(
            version: 1,
            map_id: "legacy_map",
            display_name: "Legacy",
            bounds: (min_x: -20.0, max_x: 20.0, min_z: -20.0, max_z: 20.0),
            props: [],
        )"#;
        let manifest: MapManifest = ron::from_str(source).expect("parse legacy manifest");
        assert_eq!(manifest.terrain, Terrain::default());
        assert!(validate_structure(&manifest).is_empty());
    }

    #[test]
    fn duplicate_prop_ids_are_rejected() {
        let mut m = empty_manifest();
        m.props.push(Prop {
            id: "prop_a".into(),
            kind: "tree_oak".into(),
            transform: TransformData::at(0.0, 0.0, 0.0),
            tint: None,
            collision: None,
            blocks_movement: true,
        });
        m.props.push(Prop {
            id: "prop_a".into(),
            kind: "rock_01".into(),
            transform: TransformData::at(1.0, 0.0, 0.0),
            tint: None,
            collision: None,
            blocks_movement: true,
        });
        let issues = validate_structure(&m);
        assert!(issues
            .iter()
            .any(|i| i.message.contains("duplicate prop id")));
    }

    #[test]
    fn unknown_version_is_rejected() {
        let mut m = empty_manifest();
        m.version = 99;
        let issues = validate_structure(&m);
        assert!(issues
            .iter()
            .any(|i| i.message.contains("unknown manifest version")));
    }

    #[test]
    fn json_manifest_parses_externally_tagged_collision_shapes() {
        // Mirrors the shape emitted by scripts/blender/bevymmo_export_world.py
        // `collect_props`: an externally-tagged CollisionShape plus an enemy
        // spawn marker and a player spawn marker. We assert the loader keeps
        // them in authored order and preserves the collision variant.
        let json = serde_json::json!({
            "version": 2,
            "map_id": "spawn_test",
            "display_name": "Spawn test",
            "bounds": {"min_x": -10.0, "max_x": 10.0, "min_z": -10.0, "max_z": 10.0},
            "props": [
                {
                    "id": "player_spawn_01",
                    "kind": "player_spawn",
                    "transform": {
                        "translation": [1.0, 0.0, 2.0],
                        "rotation_deg": [0.0, 0.0, 0.0],
                        "scale": [1.0, 1.0, 1.0]
                    },
                    "blocks_movement": false
                },
                {
                    "id": "mob_goblin_01",
                    "kind": "mob_goblin",
                    "transform": {
                        "translation": [-3.0, 0.0, -1.0],
                        "rotation_deg": [0.0, 90.0, 0.0],
                        "scale": [1.0, 1.0, 1.0]
                    },
                    "blocks_movement": false,
                    "collision": {"Cylinder": {"radius": 0.4, "height": 1.6}}
                }
            ]
        });

        let manifest: MapManifest =
            serde_json::from_value(json).expect("externally-tagged JSON should parse");

        assert_eq!(manifest.props.len(), 2);
        assert_eq!(manifest.props[0].kind.as_str(), "player_spawn");
        assert_eq!(manifest.props[0].collision, None);
        assert!(!manifest.props[0].blocks_movement);

        let collision = manifest.props[1]
            .collision
            .as_ref()
            .expect("goblin carries a cylinder collision");
        match collision {
            CollisionShape::Cylinder { radius, height } => {
                assert!((radius - 0.4).abs() < f32::EPSILON);
                assert!((height - 1.6).abs() < f32::EPSILON);
            }
            other => panic!("expected Cylinder, got {other:?}"),
        }
    }

    #[test]
    fn roundtrip_through_disk_preserves_manifest() {
        let mut m = empty_manifest();
        m.props.push(Prop {
            id: "prop_001".into(),
            kind: "tree_oak".into(),
            transform: TransformData {
                translation: [2.5, 0.0, -1.0],
                rotation_deg: [0.0, 45.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            tint: Some([0.1, 0.5, 0.9]),
            collision: Some(CollisionShape::Cylinder {
                radius: 0.5,
                height: 3.0,
            }),
            blocks_movement: true,
        });

        let dir = std::env::temp_dir();
        let path = dir.join("bevymmo_shared_roundtrip.ron");
        save_map(&path, &m).expect("save");
        let loaded = load_map(&path).expect("load");
        assert_eq!(m, loaded);

        // deterministic ordering: save again and confirm props are sorted by id
        m.props.push(Prop {
            id: "prop_000".into(),
            kind: "rock_01".into(),
            transform: TransformData::at(0.0, 0.0, 0.0),
            tint: None,
            collision: None,
            blocks_movement: false,
        });
        save_map(&path, &m).expect("save again");
        let body = std::fs::read_to_string(&path).expect("read");
        let pos_000 = body.find("prop_000").expect("prop_000 present");
        let pos_001 = body.find("prop_001").expect("prop_001 present");
        assert!(
            pos_000 < pos_001,
            "props should be sorted by id in the file"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn prop_outside_bounds_is_rejected() {
        let mut m = empty_manifest();
        m.props.push(Prop {
            id: "prop_x".into(),
            kind: "tree_oak".into(),
            transform: TransformData::at(100.0, 0.0, 0.0),
            tint: None,
            collision: None,
            blocks_movement: true,
        });
        let issues = validate_structure(&m);
        assert!(issues
            .iter()
            .any(|i| i.message.contains("outside map bounds")));
    }

    #[test]
    fn unknown_kind_is_flagged() {
        let mut m = empty_manifest();
        m.props.push(Prop {
            id: "prop_x".into(),
            kind: "nonexistent_kind".into(),
            transform: TransformData::at(0.0, 0.0, 0.0),
            tint: None,
            collision: None,
            blocks_movement: true,
        });
        // A default registry has no kinds registered, so any kind is unknown.
        let registry = PlaceableRegistry::default();
        let issues = validate(&m, &registry);
        assert!(issues.iter().any(|i| i.message.contains("unknown kind")));
    }

    /// Integration test: loads the current main map through auto-detection.
    ///
    /// Version 2 maps keep gameplay data in `.world.json` and visuals in the
    /// sibling `.glb`, so this intentionally exercises `load_map_auto` rather
    /// than the legacy GLB-extras loader.
    #[test]
    fn load_map_auto_prefers_map_01_sidecar() {
        let glb_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/maps/map_01.glb");

        let manifest = load_map_auto(glb_path).expect("load map_01 sidecar manifest");

        assert_eq!(manifest.map_id, "map_01");
        assert_eq!(manifest.version, 2);
        assert!(manifest.world_metrics.is_some());
        assert!(manifest
            .surfaces
            .iter()
            .any(|surface| surface.id == "surface_map_01"));
        assert!(manifest
            .test_checklist
            .iter()
            // Wording taken verbatim from map_01.world.json: the point is to
            // prove the sidecar was read, not the GLB extras.
            .any(|item| item.contains("center hill")));

        let issues = validate_structure(&manifest);
        assert!(issues.is_empty(), "validation failed: {issues:?}");
    }

    /// Guards the blocker encoding end to end on the shipped default map.
    ///
    /// `CollisionShape` is an externally tagged serde enum, so a blocker's
    /// `shape` must be a single-key map (`{"Box": {...}}`). Anything else does
    /// not degrade gracefully: `Option<CollisionShape>` fails the whole
    /// deserialization, so one malformed blocker takes the entire map down.
    /// The Blender exporter emitted `{"type": "box", ...}` until this was
    /// caught, which meant no map could ship a blocker at all.
    #[test]
    fn map_02_blockers_deserialize_and_reach_the_collision_grid() {
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/maps/map_02.world.json"
        );
        let manifest = load_world_json(json_path).expect("map_02 sidecar must load");

        assert!(
            !manifest.blockers.is_empty(),
            "map_02 is expected to ship blockers for its rocks and tree trunks"
        );
        assert!(
            manifest.blockers.iter().all(|b| b.shape.is_some()),
            "a blocker without a shape is silently dropped by CollisionGrid::build"
        );

        let grid = crate::world::CollisionGrid::build(&manifest);
        assert_eq!(
            grid.obstacle_count(),
            manifest.blockers.len(),
            "every blocking blocker must produce exactly one obstacle"
        );
    }

    /// Integration test: loads the real `.world.json` fixture for the main map.
    #[test]
    fn loads_map_01_world_json() {
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/maps/map_01.world.json"
        );
        if !std::path::Path::new(json_path).exists() {
            eprintln!("Skipping: {json_path} not found (export from Blender first)");
            return;
        }

        let manifest = load_world_json(json_path).expect("failed to load map_01.world.json");

        assert_eq!(manifest.map_id, "map_01");
        assert_eq!(manifest.version, 2);
        assert!(manifest.world_metrics.is_some());
        assert_eq!(manifest.surfaces.len(), 1);
        assert!(manifest.blockers.is_empty());

        // Check world metrics
        let metrics = manifest
            .world_metrics
            .expect("map_01 fixture should include world metrics");
        assert_eq!(metrics.player_radius, 0.35);
        assert_eq!(metrics.player_height, 1.7);
        assert_eq!(metrics.max_step_height, 0.45);
        assert_eq!(metrics.max_walkable_slope_deg, 45.0);

        let surface_query = crate::world::SurfaceQuery::from_manifest(&manifest);
        let outer_edge = surface_query
            .ground_at(-22.0, -22.0)
            .expect("outer edge should resolve from fixture heightfield");
        let central_hill = surface_query
            .ground_at(0.0, 0.0)
            .expect("central hill should resolve from fixture heightfield");
        assert!(
            central_hill.height > 2.5 && central_hill.height - outer_edge.height > 2.5,
            "central hill should be clearly above the map edge; edge={}, center={}",
            outer_edge.height,
            central_hill.height
        );

        // Validate the manifest structure
        let issues = validate_structure(&manifest);
        assert!(issues.is_empty(), "validation failed: {issues:?}");

        // Validate using the new manifest validation
        manifest
            .validate()
            .expect("manifest validation should succeed");
    }

    /// Test: load_map_auto picks GLB loader for .glb extension
    #[test]
    fn load_map_auto_detects_glb() {
        // This tests the dispatch logic; we don't need a real file for this
        // since load_map_auto just checks the extension
        let result = load_map_auto("test.glb");
        // Will fail because file doesn't exist, but error should be Gltf or Io not Ron
        match result {
            Err(MapLoadError::Gltf { .. }) => {} // correct
            Err(MapLoadError::Io { .. }) => {}   // also fine (file missing)
            other => panic!("expected Gltf or Io error, got: {other:?}"),
        }
    }
}
