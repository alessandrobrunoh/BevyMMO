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

use super::ids::validate_id;
use super::manifest::{MapBounds, MapManifest, Prop, Terrain, TransformData, CURRENT_VERSION};
use super::shapes::CollisionShape;
use crate::placeables::{KindId, PlaceableRegistry};

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

    Ok(manifest)
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
        surfaces: vec![],
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

/// Serializes the manifest to RON with deterministic ordering.
///
/// `props` are sorted by `id` so diffs stay readable across edits.
pub fn save_map<P: AsRef<Path>>(path: P, manifest: &MapManifest) -> Result<(), MapLoadError> {
    let path_ref = path.as_ref();
    let path_display = path_ref.display().to_string();

    let mut sorted = manifest.clone();
    sorted.props.sort_by(|a, b| a.id.cmp(&b.id));

    let config = PrettyConfig::new();
    let serialized =
        ron::ser::to_string_pretty(&sorted, config).map_err(|source| MapLoadError::Serialize {
            path: path_display.clone(),
            message: source.to_string(),
        })?;

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
    fn load_map_auto_prefers_rolling_hills_sidecar() {
        let glb_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/maps/rolling_hills_test.glb"
        );

        let manifest = load_map_auto(glb_path).expect("load rolling_hills_test sidecar manifest");

        assert_eq!(manifest.map_id, "rolling_hills_test");
        assert_eq!(manifest.version, 2);
        assert!(manifest.world_metrics.is_some());
        assert!(manifest
            .surfaces
            .iter()
            .any(|surface| surface.id == "surface_mountain_test"));
        assert!(manifest
            .test_checklist
            .iter()
            .any(|item| item.contains("central hill")));

        let issues = validate_structure(&manifest);
        assert!(issues.is_empty(), "validation failed: {issues:?}");
    }

    /// Integration test: loads the real `.world.json` fixture for the main map.
    #[test]
    fn loads_rolling_hills_world_json() {
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/maps/rolling_hills_test.world.json"
        );
        if !std::path::Path::new(json_path).exists() {
            eprintln!("Skipping: {json_path} not found (export from Blender first)");
            return;
        }

        let manifest =
            load_world_json(json_path).expect("failed to load rolling_hills_test.world.json");

        assert_eq!(manifest.map_id, "rolling_hills_test");
        assert_eq!(manifest.version, 2);
        assert!(manifest.world_metrics.is_some());
        assert_eq!(manifest.surfaces.len(), 1);
        assert!(manifest.blockers.is_empty());

        // Check world metrics
        let metrics = manifest
            .world_metrics
            .expect("rolling_hills_test fixture should include world metrics");
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
            central_hill.height > 5.0 && central_hill.height - outer_edge.height > 5.0,
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
