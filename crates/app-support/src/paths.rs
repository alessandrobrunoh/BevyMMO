//! Filesystem path resolution shared across the workspace.
//!
//! The runnable binary lives in `bins/game`, but it can be launched from very
//! different working directories:
//! - `cargo run` from the workspace root (cwd = repo root, exe = `target/debug/`)
//! - `cargo run` from `bins/game` (cwd = `bins/game`, exe = `target/debug/`)
//! - directly running `target/debug/game.exe` from anywhere
//! - a deployed bundle where the binary sits next to `assets/`
//!
//! Hardcoded relative paths like `"assets/maps/rolling_hills_test.glb"` or
//! `"../../assets"` only work in the first two cases and silently break in the
//! others (manifests fail to load, GLB scenes never spawn). The helpers here
//! resolve the assets folder by walking up from both the executable location
//! and the current working directory, so the same binary works everywhere.

use std::path::{Path, PathBuf};

/// Name of the assets folder at the workspace/project root.
const ASSETS_DIR_NAME: &str = "assets";

/// Default map to load when starting a local server or host-client.
///
/// Must name a map whose `.world.json` sidecar declares the same `map_id`:
/// the loader picks the sidecar by this name, while the client renders
/// `maps/<manifest.map_id>.glb`. When `map_03`'s sidecar still declared
/// `map_id = "map_02"`, the game collided against map_03's 44 m test surface
/// while drawing map_02's 360 m terrain, leaving the player apparently buried
/// nine metres under the ground they could see.
pub const DEFAULT_MAP_ID: &str = "map_02";

/// Resolves the absolute path to the workspace `assets` directory.
///
/// Search order:
/// 1. Walk up from the current executable's directory until an `assets` folder
///    is found. Covers `target/debug/game.exe` and deployed layouts where the
///    binary sits alongside `assets/`.
/// 2. Walk up from the current working directory. Covers `cargo run` launched
///    from `bins/game` or the workspace root.
/// 3. Fallback to the relative `"assets"` path to preserve legacy behavior
///    and surface a clear OS error if nothing matches.
///
/// # Performance
/// Does a small amount of filesystem `is_dir` probing; cheap enough to call
/// a handful of times at startup. Not intended for hot paths.
///
/// # Example
/// ```rust,no_run
/// let root = bevymmo_app_support::paths::assets_root();
/// let tree_glb = root.join("models/tree_oak.glb");
/// ```
pub fn assets_root() -> PathBuf {
    if let Some(found) = find_assets(walk_up_from_exe()) {
        return found;
    }
    if let Some(found) = find_assets(walk_up_from_cwd()) {
        return found;
    }
    PathBuf::from(ASSETS_DIR_NAME)
}

/// Resolves the absolute path to a map manifest inside the assets directory.
///
/// Accepts the map id without extension (e.g. `"rolling_hills_test"`) and returns
/// `<assets_root>/maps/<map_id>.glb`.
///
/// # Example
/// ```rust,no_run
/// let path = bevymmo_app_support::paths::map_file("rolling_hills_test");
/// ```
pub fn map_file(map_id: &str) -> PathBuf {
    assets_root().join("maps").join(format!("{map_id}.glb"))
}

/// Resolves the path to the default map used by client and server bootstrap.
pub fn default_map_file() -> PathBuf {
    map_file(DEFAULT_MAP_ID)
}

fn walk_up_from_exe() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

fn walk_up_from_cwd() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

fn find_assets(start: Option<PathBuf>) -> Option<PathBuf> {
    let mut current = start?;
    loop {
        let candidate = current.join(ASSETS_DIR_NAME);
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_file_appends_glb_extension_under_maps_subfolder() {
        let path = map_file("my_map");
        assert!(path.ends_with("maps/my_map.glb"));
    }

    #[test]
    fn default_map_file_points_at_map_02() {
        let path = default_map_file();
        assert!(path.ends_with("maps/map_02.glb"));
    }

    #[test]
    fn find_assets_locates_existing_folder_in_temp_tree() {
        let dir = std::env::temp_dir().join("bevymmo_paths_test");
        let nested = dir.join("a/b/c");
        std::fs::create_dir_all(&nested).expect("create nested dirs");
        std::fs::create_dir_all(dir.join("assets")).expect("create assets dir");

        let found = find_assets(Some(nested));
        assert_eq!(found, Some(dir.join("assets")));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
