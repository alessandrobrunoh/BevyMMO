//! Engine-independent world geometry, map manifests, terrain, and collision.

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
