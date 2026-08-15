//! World data, plus the disk-backed loading that only a native process can do.
//!
//! The data model itself moved to [`bevymmo_domain::world`] so the SpacetimeDB
//! module can use it — a WASM module cannot open a file, but it very much needs
//! to know what a `MapManifest` is. What stayed here is [`loader`]: glTF
//! parsing, sidecar `.world.json` files, saving from the editor.
//!
//! Everything the domain crate defines is re-exported, so callers keep writing
//! `bevymmo_shared::world::MapManifest` and never learn where it lives.

pub mod loader;

pub use bevymmo_domain::world::{
    aabb_for_shape, collision, ids, make_prop_id, manifest, shapes, validate_id, BlockerData,
    BlockerKind, CollisionGrid, CollisionShape, GroundContact, HeightfieldData, MapBounds,
    MapManifest, PlateauTest, Prop, SurfaceBounds, SurfaceKind, SurfaceQuery, SwitchbackTest,
    Terrain, TestRoutePoint, TransformData, TraversalData, TraversalKind, WalkableMeshData,
    WalkableSurface, WorldMetrics, CURRENT_VERSION, LEGACY_VERSION_1,
};
pub use loader::{
    has_world_json_sidecar, load_map, load_map_auto, load_map_from_glb, load_world_json, save_map,
    validate, MapLoadError, ValidationIssue,
};
