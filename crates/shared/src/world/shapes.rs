//! Server-side collision primitives. Pure math, no Bevy dependencies.

use serde::{Deserialize, Serialize};

/// Cheap-to-test collision shapes for movement validation.
///
/// The same enum lives in the manifest (via `Prop.collision`) and is consumed
/// identically by the server (authoritative validation) and the client
/// (prediction), so the data model must stay engine-agnostic.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum CollisionShape {
    /// Vertical cylinder, axis on Y.
    Cylinder { radius: f32, height: f32 },
    /// Axis-aligned box (no rotation in v1; rotation is applied via the
    /// prop's `transform`, but the collision test uses the world AABB).
    Box { half_extents: [f32; 3] },
    /// Sphere (collision center is the prop translation).
    Sphere { radius: f32 },
}

impl CollisionShape {
    /// Conservative axis-aligned bounding box for this shape, in local space.
    /// Rotation is ignored in v1 (the editor rotates visuals; the collision
    /// grid uses AABBs to keep O(1) neighbor lookups).
    pub fn local_aabb(&self) -> [f32; 3] {
        match *self {
            CollisionShape::Cylinder { radius, height } => [radius, height * 0.5, radius],
            CollisionShape::Box { half_extents } => half_extents,
            CollisionShape::Sphere { radius } => [radius, radius, radius],
        }
    }
}

/// Returns the world-space AABB (min, max) for a shape placed at `position`.
pub fn aabb_for_shape(position: [f32; 3], shape: CollisionShape) -> ([f32; 3], [f32; 3]) {
    let h = shape.local_aabb();
    let min = [position[0] - h[0], position[1] - h[1], position[2] - h[2]];
    let max = [position[0] + h[0], position[1] + h[1], position[2] + h[2]];
    (min, max)
}
