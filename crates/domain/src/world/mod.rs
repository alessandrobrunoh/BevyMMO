//! Engine-agnostic world data: the contract between editor, server, and client.
//!
//! Serializable structs and pure functions only — no Bevy systems, no
//! `AssetServer`, no rendering, and (unlike the loader that used to live
//! alongside them) no filesystem. That last point is what lets the SpacetimeDB
//! module use them: a WASM module has no disk to read a map from, so it receives
//! this data pre-decoded and works on it in memory.
//!
//! Reading maps off disk is the client's and the editor's job, and stayed
//! behind in `bevymmo_shared::world::loader`.

pub mod collision;
pub mod ids;
pub mod manifest;
pub mod shapes;

pub use collision::{CollisionGrid, GroundContact, SurfaceQuery};
pub use ids::{make_prop_id, validate_id};
pub use manifest::{
    BlockerData, BlockerKind, HeightfieldData, MapBounds, MapManifest, PlateauTest, Prop,
    SurfaceBounds, SurfaceKind, SwitchbackTest, Terrain, TestRoutePoint, TransformData,
    TraversalData, TraversalKind, WalkableMeshData, WalkableSurface, WorldMetrics, CURRENT_VERSION,
    LEGACY_VERSION_1,
};
pub use shapes::{aabb_for_shape, CollisionShape};
