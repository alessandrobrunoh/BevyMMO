//! Runtime persistence merge layer for placed props.
//!
//! At server startup the world plugin loads the static [`MapManifest`] from
//! disk into [`ServerWorldMap`]. This module runs **once**, immediately after
//! that load, and folds persisted runtime overrides on top of the in-memory
//! manifest before any spawn system reads it. The merge is one-directional
//! (DB -> manifest); a write-back GM-edit surface is future work.
//!
//! Merge semantics (see [`apply_overrides`]):
//!   * `removed_at` set  -> drop the matching prop from the manifest.
//!   * `transform_json`  -> replace the prop's `TransformData`.
//!   * `tint`            -> replace the prop's `tint`.
//! Overrides that reference a `prop_id` absent from the manifest are skipped
//! (override-only creations are out of scope for this starter slice).

use bevy::prelude::*;

use bevymmo_shared::world::{MapManifest, TransformData};

use crate::persistence::entity::prop_override::Model as PropOverrideModel;
use crate::persistence::{PersistenceRuntime, PropOverrideStore};
use crate::world::ServerWorldMap;

/// Run-once guard: the prop-override merge runs the first frame the map is
/// loaded and persistence is available, then inserts this resource so it never
/// runs again.
#[derive(Resource, Default, Debug)]
pub struct PropOverridesApplied;

/// Folds persisted prop overrides into the in-memory map manifest.
///
/// Removals (`removed_at` is set) take precedence and erase the prop entirely;
/// for surviving props, `transform_json` and `tint` are applied in place.
/// Overrides whose `prop_id` does not match a manifest prop are ignored —
/// override-only prop creation is intentionally not supported by this slice.
///
/// `map_id` is accepted for context/logging only; callers are expected to have
/// already filtered overrides to the current map (e.g. via
/// [`PropOverrideRepository::list_for_map`]).
///
/// [`PropOverrideRepository::list_for_map`]: crate::persistence::repository::prop_override::PropOverrideRepository::list_for_map
pub fn apply_overrides(
    manifest: &mut MapManifest,
    overrides: &[PropOverrideModel],
    map_id: &str,
) {
    let removed: std::collections::HashSet<&str> = overrides
        .iter()
        .filter(|o| o.removed_at.is_some())
        .map(|o| o.prop_id.as_str())
        .collect();
    if !removed.is_empty() {
        manifest.props.retain(|p| !removed.contains(p.id.as_str()));
    }

    for override_row in overrides.iter().filter(|o| o.removed_at.is_none()) {
        let Some(prop) = manifest.props.iter_mut().find(|p| p.id == override_row.prop_id)
        else {
            // Override targets a prop that isn't in this manifest (e.g. stale
            // row from a previous map revision, or already removed above).
            warn!(
                map_id,
                prop_id = %override_row.prop_id,
                "prop override references unknown prop; skipping"
            );
            continue;
        };

        if let Some(json) = override_row.transform_json.as_deref() {
            match serde_json::from_str::<TransformData>(json) {
                Ok(transform) => prop.transform = transform,
                Err(error) => error!(
                    map_id,
                    prop_id = %override_row.prop_id,
                    "failed to parse transform_json override; leaving transform unchanged: {error}"
                ),
            }
        }

        if let Some(tint) = override_row.tint.as_ref() {
            match serde_json::from_value::<[f32; 3]>(tint.clone()) {
                Ok(rgb) => prop.tint = Some(rgb),
                Err(error) => error!(
                    map_id,
                    prop_id = %override_row.prop_id,
                    "failed to parse tint override; leaving tint unchanged: {error}"
                ),
            }
        }
    }
}

/// Run-once system that loads persisted prop overrides and merges them into
/// the in-memory [`ServerWorldMap`] before creature spawn reads it.
///
/// Runs on the dedicated [`PersistenceRuntime`] via `block_on` so the manifest
/// is fully patched within a single frame; ordering `.before(
/// spawn_placeables_on_map_load)` guarantees spawns see the merged state. If
/// persistence is disabled (no [`PropOverrideStore`] resource — e.g. in tests)
/// the system records the run-once guard and exits without touching anything.
// TODO: collision grid is not rebuilt after a transform/remove override; revisit
// once the override surface grows beyond the starter scope.
pub fn apply_prop_overrides_on_map_load(
    mut commands: Commands,
    mut world_map: ResMut<ServerWorldMap>,
    store: Option<Res<PropOverrideStore>>,
    runtime: Res<PersistenceRuntime>,
) {
    if let Some(store) = store {
        let map_id = world_map.manifest.map_id.clone();
        let overrides = runtime.0.block_on(async { store.0.list_for_map(&map_id).await });

        match overrides {
            Ok(rows) => {
                let count = rows.len();
                apply_overrides(&mut world_map.manifest, &rows, &map_id);
                info!(map_id = %map_id, applied = count, "applied persisted prop overrides");
            }
            Err(error) => {
                error!(
                    map_id = %map_id,
                    "failed to load prop overrides; proceeding with static manifest: {error}"
                );
            }
        }
    } else {
        debug!("persistence disabled; skipping prop override merge");
    }

    commands.insert_resource(PropOverridesApplied);
}
