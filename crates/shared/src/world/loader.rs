//! RON loader/saver and manifest validation.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use ron::ser::PrettyConfig;
use thiserror::Error;

use super::ids::validate_id;
use super::manifest::{MapBounds, MapManifest, CURRENT_VERSION};

#[derive(Debug, Error)]
pub enum MapLoadError {
    #[error("failed to read map file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse map file {path}: {source}")]
    Ron {
        path: String,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("failed to serialize map file {path}: {message}")]
    Serialize { path: String, message: String },
    #[error("manifest validation failed: {0:?}")]
    Validation(Vec<ValidationIssue>),
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

    let issues = validate(&manifest);
    if !issues.is_empty() {
        return Err(MapLoadError::Validation(issues));
    }

    Ok(manifest)
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

/// Returns all validation issues found in `manifest`. An empty result means
/// the manifest is valid.
pub fn validate(manifest: &MapManifest) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if manifest.version != CURRENT_VERSION {
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
        if prop.kind.trim().is_empty() {
            issues.push(ValidationIssue::new(format!(
                "prop {:?} has empty kind",
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
        }
    }

    #[test]
    fn empty_manifest_is_valid() {
        assert!(validate(&empty_manifest()).is_empty());
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
        assert!(validate(&manifest).is_empty());
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
        let issues = validate(&m);
        assert!(issues
            .iter()
            .any(|i| i.message.contains("duplicate prop id")));
    }

    #[test]
    fn unknown_version_is_rejected() {
        let mut m = empty_manifest();
        m.version = 99;
        let issues = validate(&m);
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
        let issues = validate(&m);
        assert!(issues
            .iter()
            .any(|i| i.message.contains("outside map bounds")));
    }
}
