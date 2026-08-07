//! Engine-agnostic world data: the contract between editor, server, and client.
//!
//! Contains only serializable structs and pure functions (no Bevy systems,
//! no `AssetServer`, no rendering). This guarantees the module compiles on
//! the headless server and on the client.

pub mod collision;
pub mod ids;
pub mod loader;
pub mod manifest;
pub mod shapes;

pub use collision::CollisionGrid;
pub use ids::{make_prop_id, validate_id};
pub use loader::{load_map, save_map, validate, MapLoadError, ValidationIssue};
pub use manifest::{MapBounds, MapManifest, Prop, Terrain, TransformData, CURRENT_VERSION};
pub use shapes::{aabb_for_shape, CollisionShape};
