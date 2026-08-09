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

pub use collision::{CollisionGrid, GroundContact, SurfaceQuery};
pub use ids::{make_prop_id, validate_id};
pub use loader::{
    has_world_json_sidecar, load_map, load_map_auto, load_map_from_glb, load_world_json, save_map,
    validate, MapLoadError, ValidationIssue,
};
pub use manifest::{
    BlockerData, BlockerKind, HeightfieldData, MapBounds, MapManifest, PlateauTest, Prop,
    SurfaceBounds, SurfaceKind, SwitchbackTest, Terrain, TestRoutePoint, TransformData,
    TraversalData, TraversalKind, WalkableSurface, WorldMetrics, CURRENT_VERSION, LEGACY_VERSION_1,
};
pub use shapes::{aabb_for_shape, CollisionShape};
