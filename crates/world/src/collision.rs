//! Shared, lightweight world collision and ground queries.
//!
//! This module provides:
//! - AABB broad-phase collision for obstacles (blocking props)
//! - Ground contact resolution for height-aware movement
//! - Surface query functionality for walkable surfaces
//!
//! All math is identical on server/client to ensure movement prediction matches
//! authoritative server validation.

use std::collections::HashMap;

use super::manifest::{HeightfieldData, MapManifest, SurfaceKind, WalkableSurface};
use super::shapes::{aabb_for_shape, CollisionShape};

use crate::manifest::WalkableMeshData;

/// Vertical slack used when rejecting obstacles whose Y range sits clearly
/// below the player.
///
/// Without this tolerance a player standing a few centimetres above a rock
/// (e.g. due to ground-snap rounding) would slip past the very obstacle it
/// should be colliding with. We keep the slack tight (smaller than the
/// player radius) so that a rock on the ground cannot block a player walking
/// on a platform a couple of metres above.
pub const OBSTACLE_Y_TOLERANCE: f32 = 0.25;

#[derive(Clone, Copy, Debug)]
struct Obstacle {
    min: [f32; 3],
    max: [f32; 3],
}

/// Side of one spatial hash cell, in world units.
const COLLISION_CELL_SIZE: f32 = 8.0;

#[derive(Clone, Debug, Default)]
pub struct CollisionGrid {
    obstacles: Vec<Obstacle>,
    /// Obstacle indices overlapping each XZ cell.
    cells: HashMap<(i32, i32), Vec<u32>>,
}

/// Axis-aligned bounding box of `shape` at `translation`, scaled about its
/// own center by (the absolute value of) `scale`.
///
/// Shared by both loops in [`CollisionGrid::build`]: props and blockers are
/// different manifest entries, but "unscaled AABB, then stretch each axis
/// around its center by the transform's scale" is the same computation for
/// both.
fn scaled_aabb(translation: [f32; 3], scale: [f32; 3], shape: CollisionShape) -> Obstacle {
    let (mut min, mut max) = aabb_for_shape(translation, shape);
    let scale = scale.map(f32::abs);
    for axis in 0..3 {
        let center = translation[axis];
        let half_extent = (max[axis] - min[axis]) * 0.5 * scale[axis];
        min[axis] = center - half_extent;
        max[axis] = center + half_extent;
    }
    Obstacle { min, max }
}

fn collision_cell(value: f32) -> i32 {
    (value / COLLISION_CELL_SIZE).floor() as i32
}

fn circle_hits_aabb(point: [f32; 3], radius: f32, obstacle: &Obstacle) -> bool {
    if point[1] > obstacle.max[1] + OBSTACLE_Y_TOLERANCE {
        return false;
    }
    let closest_x = point[0].clamp(obstacle.min[0], obstacle.max[0]);
    let closest_z = point[2].clamp(obstacle.min[2], obstacle.max[2]);
    let dx = point[0] - closest_x;
    let dz = point[2] - closest_z;
    dx * dx + dz * dz <= radius * radius
}

impl CollisionGrid {
    pub fn build(manifest: &MapManifest) -> Self {
        let mut grid = Self::default();

        for prop in &manifest.props {
            let Some(shape) = prop.collision else {
                continue;
            };
            if !prop.blocks_movement {
                continue;
            }
            grid.push_obstacle(scaled_aabb(
                prop.transform.translation,
                prop.transform.scale,
                shape,
            ));
        }

        for blocker in &manifest.blockers {
            let Some(transform) = &blocker.transform else {
                continue;
            };
            let Some(shape) = blocker.shape else {
                continue;
            };
            if !blocker.blocks_movement {
                continue;
            }
            grid.push_obstacle(scaled_aabb(transform.translation, transform.scale, shape));
        }

        grid
    }

    fn push_obstacle(&mut self, obstacle: Obstacle) {
        let index = self.obstacles.len() as u32;
        let min_x = collision_cell(obstacle.min[0]);
        let max_x = collision_cell(obstacle.max[0]);
        let min_z = collision_cell(obstacle.min[2]);
        let max_z = collision_cell(obstacle.max[2]);
        for cell_x in min_x..=max_x {
            for cell_z in min_z..=max_z {
                self.cells.entry((cell_x, cell_z)).or_default().push(index);
            }
        }
        self.obstacles.push(obstacle);
    }

    /// Returns true when a player circle in the X/Z plane intersects a
    /// blocking obstacle.
    ///
    /// The Y axis is honoured with [`OBSTACLE_Y_TOLERANCE`] of slack: an
    /// obstacle whose top sits more than the tolerance below the query point
    /// is treated as ground-floor clutter and ignored by an entity on a
    /// higher platform directly above. This is what prevents a rock on the
    /// floor from blocking a player walking on a bridge over it.
    ///
    /// Entities at or below the obstacle's top (e.g. standing next to a wall
    /// on the same floor) are still blocked as before, so existing flat-map
    /// behaviour is preserved.
    pub fn is_blocked(&self, point: [f32; 3], radius: f32) -> bool {
        if self.obstacles.is_empty() {
            return false;
        }
        let radius = radius.max(0.0);
        let min_x = collision_cell(point[0] - radius);
        let max_x = collision_cell(point[0] + radius);
        let min_z = collision_cell(point[2] - radius);
        let max_z = collision_cell(point[2] + radius);
        for cell_x in min_x..=max_x {
            for cell_z in min_z..=max_z {
                let Some(indices) = self.cells.get(&(cell_x, cell_z)) else {
                    continue;
                };
                for &index in indices {
                    if circle_hits_aabb(point, radius, &self.obstacles[index as usize]) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn obstacle_count(&self) -> usize {
        self.obstacles.len()
    }
}

// ==================== GROUND CONTACT RESOLUTION ====================

/// Result of a ground height query at a world position.
///
/// Contains both the height (y-coordinate) of the ground and the surface
/// normal, which is needed for proper movement validation and camera placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundContact {
    /// Ground height in world units at the query position.
    pub height: f32,
    /// Surface normal vector (normalized). Points "up" from the ground.
    pub normal: [f32; 3],
}

impl GroundContact {
    /// Creates a new ground contact with the given height and normal.
    pub fn new(height: f32, normal: [f32; 3]) -> Self {
        Self { height, normal }
    }

    /// Creates a flat ground contact with horizontal normal (0, 1, 0).
    pub fn flat(height: f32) -> Self {
        Self {
            height,
            normal: [0.0, 1.0, 0.0],
        }
    }
}

/// Surface query system for height-aware movement.
///
/// Provides ground height resolution for walkable surfaces defined in the
/// map manifest. For version 2 maps, this includes flat surfaces, mesh-based
/// surfaces, and traversal objects.
#[derive(Clone, Debug)]
pub struct SurfaceQuery {
    /// Walkable surfaces from the manifest.
    surfaces: Vec<WalkableSurface>,
    /// Global maximum walkable slope in degrees.
    global_max_slope_deg: f32,
}

impl SurfaceQuery {
    /// Creates a new surface query system from a map manifest.
    ///
    /// Extracts walkable surface data and prepares it for queries.
    /// For version 1 maps (without surfaces), this returns an empty query system.
    pub fn from_manifest(manifest: &MapManifest) -> Self {
        Self {
            surfaces: manifest.surfaces.clone(),
            global_max_slope_deg: manifest.get_world_metrics().max_walkable_slope_deg,
        }
    }

    /// Resolves ground contact at the given world position using highest-wins.
    ///
    /// Returns `None` if the position is not over any walkable surface.
    /// When multiple surfaces overlap, returns the highest one. For stepping an
    /// entity that is already standing somewhere, use [`ground_at_reachable`].
    /// For raycasts, targeting and cursor aiming on any terrain (including steep
    /// slopes and cliffs), use [`surface_contact_at`].
    ///
    /// [`ground_at_reachable`]: SurfaceQuery::ground_at_reachable
    /// [`surface_contact_at`]: SurfaceQuery::surface_contact_at
    pub fn ground_at(&self, x: f32, z: f32) -> Option<GroundContact> {
        self.surfaces
            .iter()
            .filter(|surface| self.surface_contains_point(surface, x, z))
            .filter_map(|surface| self.resolve_surface(surface, x, z, true))
            .max_by(|left, right| left.height.total_cmp(&right.height))
    }

    /// Resolves ground contact at (x, z) without slope rejection.
    ///
    /// Unlike [`ground_at`], which rejects surfaces with slope steeper than the
    /// walkable limit for entity foot movement, `surface_contact_at` returns the
    /// physical terrain surface height for raycasts, cursor aiming, and spell targeting.
    pub fn surface_contact_at(&self, x: f32, z: f32) -> Option<GroundContact> {
        self.surfaces
            .iter()
            .filter(|surface| self.surface_contains_point(surface, x, z))
            .filter_map(|surface| self.resolve_surface(surface, x, z, false))
            .max_by(|left, right| left.height.total_cmp(&right.height))
    }

    /// Resolves the highest surface at `(x, z)` that the entity can step
    /// onto from `current_y`, applying an **asymmetric** reachability rule:
    ///
    /// - Surfaces **above** `current_y + max_step_height` are filtered out.
    ///   This is what prevents a single tick from teleporting the entity up
    ///   a cliff or onto an unrelated overlapping surface (the
    ///   "sale in punti strani" bug).
    /// - Surfaces **at or below** `current_y + max_step_height` are always
    ///   kept. This deliberately allows the entity to step *down* by any
    ///   amount (there is no "fall damage" here) and, crucially, lets an
    ///   entity that has been stranded above its surface (e.g. by spawn,
    ///   respawn, teleport or knockback) snap back down onto it. Without
    ///   this asymmetry the entity would be permanently stuck the first time
    ///   its Y drifts more than `max_step_height` away from every surface.
    ///
    /// Among the reachable candidates, the highest one wins. This lets the
    /// entity climb ramps and switchbacks one step per tick while naturally
    /// staying on the current surface when no higher neighbour is reachable.
    pub fn ground_at_reachable(
        &self,
        x: f32,
        z: f32,
        current_y: f32,
        max_step_height: f32,
    ) -> Option<GroundContact> {
        let ceiling = current_y + max_step_height;
        self.surfaces
            .iter()
            .filter(|surface| self.surface_contains_point(surface, x, z))
            .filter_map(|surface| self.resolve_surface(surface, x, z, true))
            .filter(|contact| contact.height <= ceiling)
            .max_by(|left, right| left.height.total_cmp(&right.height))
    }

    /// Checks if a surface contains a given 2D point (x, z).
    ///
    /// For surfaces with walkable_mesh data, this is a broad-phase check only.
    /// The exact triangle containment test is performed in resolve_surface.
    fn surface_contains_point(&self, surface: &WalkableSurface, x: f32, z: f32) -> bool {
        match surface.kind {
            SurfaceKind::Flat => {
                if let Some(bounds) = surface.bounds {
                    return bounds.contains(x, z);
                }
                false
            }
            SurfaceKind::FlatMesh => {
                // For flat_mesh, check heightfield bounds if available
                if let Some(ref heightfield) = surface.heightfield {
                    return heightfield.bounds.contains(x, z);
                }
                // Fallback to surface bounds
                if let Some(bounds) = surface.bounds {
                    return bounds.contains(x, z);
                }
                false
            }
            SurfaceKind::Mesh => {
                // For mesh surfaces with walkable_mesh data, use bounds as broad-phase
                if surface.walkable_mesh.is_some() {
                    if let Some(ref heightfield) = surface.heightfield {
                        return heightfield.bounds.contains(x, z);
                    }
                    if let Some(bounds) = surface.bounds {
                        return bounds.contains(x, z);
                    }
                    return false;
                }
                // For mesh surfaces without walkable_mesh, check heightfield bounds if available
                if let Some(ref heightfield) = surface.heightfield {
                    return heightfield.bounds.contains(x, z);
                }
                // Fallback to surface bounds
                if let Some(bounds) = surface.bounds {
                    return bounds.contains(x, z);
                }
                false
            }
        }
    }

    /// Resolves ground contact for a triangle mesh surface.
    ///
    /// Performs an exact point-in-triangle test on the X/Z plane and computes
    /// the interpolated height and surface normal using barycentric coordinates.
    fn resolve_triangle_mesh(
        &self,
        mesh: &WalkableMeshData,
        surface: &WalkableSurface,
        x: f32,
        z: f32,
        check_slope: bool,
    ) -> Option<GroundContact> {
        let min_normal_y = if check_slope {
            let max_slope = surface.max_slope_deg.unwrap_or(self.global_max_slope_deg);
            let max_slope_rad = max_slope.to_radians();
            Some(max_slope_rad.cos() - 1e-4) // Tolerate precision errors
        } else {
            None
        };

        let tri_count = mesh.indices.len() / 3;
        for tri_idx in 0..tri_count {
            let i0 = mesh.indices[tri_idx * 3] as usize;
            let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
            let i2 = mesh.indices[tri_idx * 3 + 2] as usize;

            let v0 = mesh.vertices[i0];
            let v1 = mesh.vertices[i1];
            let v2 = mesh.vertices[i2];

            // Check if point (x, z) is inside triangle on X/Z plane, and get
            // its barycentric weights in the same pass (both used to need
            // the same v0/v1/v2/dot*/inv_denom/u/v math, computed twice on
            // every triangle that actually contained the point).
            let p = [x, z];
            let (inside, (w0, w1, w2)) = Self::triangle_containment_and_barycentric(
                p,
                [v0[0], v0[2]],
                [v1[0], v1[2]],
                [v2[0], v2[2]],
            );
            if !inside {
                continue;
            }

            // Compute triangle normal
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let mut normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];

            // Normalize
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if length < 1e-6 {
                continue; // Degenerate triangle
            }
            normal = [normal[0] / length, normal[1] / length, normal[2] / length];

            // Check slope if requested: normal should point up (positive Y)
            if let Some(min_y) = min_normal_y {
                if normal[1] < min_y {
                    continue; // Too steep
                }
            }

            // Interpolate height
            let height = w0 * v0[1] + w1 * v1[1] + w2 * v2[1];

            return Some(GroundContact::new(height, normal));
        }

        None // No triangle contains the point
    }

    /// Checks if a point `(x, z)` is inside a triangle on the X/Z plane and,
    /// in the same pass, computes its barycentric weights.
    ///
    /// Returns `(inside, (w_a, w_b, w_c))`. `inside` uses the
    /// sign-of-cross-product method: the point is inside when it lies on
    /// the same side of all three edges, with a small epsilon to tolerate
    /// floating point inaccuracies on triangle edges — this prevents the
    /// player from getting stuck on "invisible walls" when crossing
    /// boundaries between adjacent triangles in a mesh surface. `(w_a, w_b,
    /// w_c)` are the weights for vertices `a`, `b`, `c` respectively and sum
    /// to 1.0 inside the triangle.
    ///
    /// Both results fall out of the same `dot00`/`dot01`/`dot02`/`dot11`/
    /// `dot12`/`inv_denom`/`u`/`v` computation, which used to be duplicated
    /// across two separate functions that `resolve_triangle_mesh` called
    /// back to back on every triangle that contained the point.
    fn triangle_containment_and_barycentric(
        p: [f32; 2],
        a: [f32; 2],
        b: [f32; 2],
        c: [f32; 2],
    ) -> (bool, (f32, f32, f32)) {
        // Vectors relative to vertex `a`.
        let v0 = [c[0] - a[0], c[1] - a[1]];
        let v1 = [b[0] - a[0], b[1] - a[1]];
        let v2 = [p[0] - a[0], p[1] - a[1]];

        let dot00 = v0[0] * v0[0] + v0[1] * v0[1];
        let dot01 = v0[0] * v1[0] + v0[1] * v1[1];
        let dot02 = v0[0] * v2[0] + v0[1] * v2[1];
        let dot11 = v1[0] * v1[0] + v1[1] * v1[1];
        let dot12 = v1[0] * v2[0] + v1[1] * v2[1];

        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
        let w = 1.0 - u - v;

        let epsilon = 1e-4;
        let inside = (u >= -epsilon) && (v >= -epsilon) && (u + v <= 1.0 + epsilon);

        (inside, (w, u, v))
    }

    /// Resolves ground contact for a specific surface.
    fn resolve_surface(
        &self,
        surface: &WalkableSurface,
        x: f32,
        z: f32,
        check_slope: bool,
    ) -> Option<GroundContact> {
        match surface.kind {
            SurfaceKind::Flat => surface.height.map(GroundContact::flat),
            SurfaceKind::FlatMesh => {
                // For flat_mesh, try heightfield data first, then fall back to constant height
                if let Some(ref heightfield) = surface.heightfield {
                    self.resolve_heightfield(surface, heightfield, x, z, check_slope)
                } else {
                    surface.height.map(GroundContact::flat)
                }
            }
            SurfaceKind::Mesh => {
                // For mesh surfaces with walkable_mesh data, perform exact triangle test
                if let Some(ref mesh) = surface.walkable_mesh {
                    return self.resolve_triangle_mesh(mesh, surface, x, z, check_slope);
                }
                // For mesh surfaces, use heightfield data if available
                if let Some(ref heightfield) = surface.heightfield {
                    self.resolve_heightfield(surface, heightfield, x, z, check_slope)
                } else {
                    // No heightfield data available
                    None
                }
            }
        }
    }

    /// Resolves a heightfield-backed surface and optionally applies the walkable slope limit.
    fn resolve_heightfield(
        &self,
        surface: &WalkableSurface,
        heightfield: &HeightfieldData,
        x: f32,
        z: f32,
        check_slope: bool,
    ) -> Option<GroundContact> {
        let height = heightfield.sample_height(x, z)?;
        let normal = heightfield.sample_normal(x, z)?;
        if check_slope {
            let max_slope = surface.max_slope_deg.unwrap_or(self.global_max_slope_deg);
            let min_normal_y = max_slope.to_radians().cos() - 1e-4; // Tolerate precision errors
            if normal[1] < min_normal_y {
                return None;
            }
        }

        Some(GroundContact::new(height, normal))
    }

    /// Returns the number of surfaces in the query system.
    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }

    /// Returns true if the surface query system is empty (no walkable surfaces).
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HeightfieldData, MapBounds, MapManifest, SurfaceBounds, SurfaceKind, WalkableSurface,
        WorldMetrics, CURRENT_VERSION,
    };

    #[test]
    fn test_ground_contact_flat_surface() {
        let ground = GroundContact::flat(2.5);
        assert_eq!(ground.height, 2.5);
        assert_eq!(ground.normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn scaled_aabb_stretches_the_unscaled_box_around_its_center() {
        let unscaled = scaled_aabb(
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            CollisionShape::Box {
                half_extents: [1.0, 1.0, 1.0],
            },
        );
        let doubled = scaled_aabb(
            [0.0, 0.0, 0.0],
            [2.0, 2.0, 2.0],
            CollisionShape::Box {
                half_extents: [1.0, 1.0, 1.0],
            },
        );
        assert_eq!(unscaled.min, [-1.0, -1.0, -1.0]);
        assert_eq!(unscaled.max, [1.0, 1.0, 1.0]);
        assert_eq!(doubled.min, [-2.0, -2.0, -2.0]);
        assert_eq!(doubled.max, [2.0, 2.0, 2.0]);
    }

    #[test]
    fn scaled_aabb_recenters_on_a_translated_origin() {
        let obstacle = scaled_aabb(
            [5.0, 0.0, -3.0],
            [1.0, 1.0, 1.0],
            CollisionShape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
        );
        assert_eq!(obstacle.min, [4.5, -0.5, -3.5]);
        assert_eq!(obstacle.max, [5.5, 0.5, -2.5]);
    }

    #[test]
    fn scaled_aabb_treats_a_negative_scale_the_same_as_its_absolute_value() {
        let positive = scaled_aabb(
            [0.0, 0.0, 0.0],
            [3.0, 1.0, 1.0],
            CollisionShape::Box {
                half_extents: [1.0, 1.0, 1.0],
            },
        );
        let negative = scaled_aabb(
            [0.0, 0.0, 0.0],
            [-3.0, 1.0, 1.0],
            CollisionShape::Box {
                half_extents: [1.0, 1.0, 1.0],
            },
        );
        assert_eq!(positive.min, negative.min);
        assert_eq!(positive.max, negative.max);
    }

    #[test]
    fn triangle_containment_reports_a_point_inside_and_its_barycentric_weights() {
        let (inside, (w_a, w_b, w_c)) = SurfaceQuery::triangle_containment_and_barycentric(
            [1.0, 1.0],
            [0.0, 0.0],
            [3.0, 0.0],
            [0.0, 3.0],
        );
        assert!(inside);
        // Weights sum to 1 and interpolate the vertices back to the point.
        assert!((w_a + w_b + w_c - 1.0).abs() < 1e-5);
        assert!(w_a > 0.0 && w_b > 0.0 && w_c > 0.0);
    }

    #[test]
    fn triangle_containment_rejects_a_point_clearly_outside() {
        let (inside, _) = SurfaceQuery::triangle_containment_and_barycentric(
            [10.0, 10.0],
            [0.0, 0.0],
            [3.0, 0.0],
            [0.0, 3.0],
        );
        assert!(!inside);
    }

    #[test]
    fn triangle_containment_accepts_a_point_exactly_on_an_edge() {
        // Midpoint of the edge from (0,0) to (3,0): must count as inside so
        // a point sitting on the shared edge between two adjacent triangles
        // in a mesh does not fall through both.
        let (inside, (w_a, w_b, w_c)) = SurfaceQuery::triangle_containment_and_barycentric(
            [1.5, 0.0],
            [0.0, 0.0],
            [3.0, 0.0],
            [0.0, 3.0],
        );
        assert!(inside);
        assert!((w_a - 0.5).abs() < 1e-4);
        assert!(w_b.abs() < 1e-4);
        assert!(w_c > 0.4 && w_c < 0.6);
    }

    #[test]
    fn test_surface_query_empty_manifest() {
        let manifest = create_test_manifest_v1();
        let query = SurfaceQuery::from_manifest(&manifest);

        assert!(query.is_empty());
        assert_eq!(query.surface_count(), 0);
        assert!(query.ground_at(0.0, 0.0).is_none());
    }

    #[test]
    fn test_surface_query_flat_surface() {
        let manifest = create_test_manifest_with_flat_surface();
        let query = SurfaceQuery::from_manifest(&manifest);

        // Query inside the flat surface bounds
        let contact = query
            .ground_at(5.0, 5.0)
            .expect("flat test surface should resolve at an in-bounds point");
        assert_eq!(contact.height, 1.5);

        // Query outside the surface bounds
        let contact = query.ground_at(100.0, 100.0);
        assert!(contact.is_none());
    }

    #[test]
    fn test_surface_query_multiple_surfaces() {
        let manifest = create_test_manifest_with_multiple_surfaces();
        let query = SurfaceQuery::from_manifest(&manifest);

        assert_eq!(query.surface_count(), 2);

        // Query on surface 1
        let contact = query
            .ground_at(5.0, 5.0)
            .expect("first flat surface should resolve at an in-bounds point");
        assert_eq!(contact.height, 1.5);

        // Query on surface 2
        let contact = query
            .ground_at(15.0, 15.0)
            .expect("second flat surface should resolve at an in-bounds point");
        assert_eq!(contact.height, 3.0);
    }

    #[test]
    fn test_heightfield_basic() {
        let bounds = SurfaceBounds {
            min_x: 0.0,
            max_x: 10.0,
            min_z: 0.0,
            max_z: 10.0,
        };

        // Create a simple 2x2 heightfield
        let heights = vec![
            1.0, 1.2, 1.4, // Column 0 (x=0; z varies fastest)
            1.5, 1.7, 1.9, // Column 1 (x=5)
            2.0, 2.2, 2.4, // Column 2 (x=10)
        ];

        let heightfield = HeightfieldData::new(2, bounds, heights);
        assert!(heightfield.validate().is_ok());

        // Test corner sampling
        assert_eq!(heightfield.sample_height(0.0, 0.0), Some(1.0));
        assert_eq!(heightfield.sample_height(10.0, 0.0), Some(2.0));
        assert_eq!(heightfield.sample_height(0.0, 10.0), Some(1.4));
        assert_eq!(heightfield.sample_height(10.0, 10.0), Some(2.4));

        // Test center sampling (bilinear interpolation)
        let center_height = heightfield.sample_height(5.0, 5.0);
        assert!(center_height.is_some());
        // Center should be roughly average of the four corners
        let h = center_height.expect("center point should be inside the heightfield");
        assert!(
            h > 1.6 && h < 1.8,
            "Center height {} should be around 1.7",
            h
        );

        let normal = heightfield
            .sample_normal(5.0, 5.0)
            .expect("center point should have an estimated normal");
        assert!(
            normal[1] > 0.99,
            "gentle test slope should remain mostly upward"
        );

        // Test outside bounds
        assert_eq!(heightfield.sample_height(-1.0, 5.0), None);
        assert_eq!(heightfield.sample_height(11.0, 5.0), None);
    }

    #[test]
    fn heightfield_query_rejects_surface_above_walkable_slope() {
        let bounds = SurfaceBounds {
            min_x: 0.0,
            max_x: 1.0,
            min_z: 0.0,
            max_z: 1.0,
        };
        let heightfield = HeightfieldData::new(
            1,
            bounds,
            vec![
                0.0, 0.0, // x=0
                10.0, 10.0, // x=1: intentionally cliff-like
            ],
        );
        let manifest = MapManifest {
            version: 2,
            map_id: "steep_heightfield".to_string(),
            display_name: "Steep Heightfield".to_string(),
            bounds: MapBounds {
                min_x: 0.0,
                max_x: 1.0,
                min_z: 0.0,
                max_z: 1.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![WalkableSurface {
                id: "steep".to_string(),
                kind: SurfaceKind::Mesh,
                object: None,
                bounds: None,
                height: None,
                min_height: None,
                max_height: None,
                grid_size: Some(1),
                size: Some(1.0),
                purpose: None,
                heightfield: Some(heightfield),
                walkable_mesh: None,
                layer: None,
                max_slope_deg: Some(45.0),
            }],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let query = SurfaceQuery::from_manifest(&manifest);
        assert!(query.ground_at(0.5, 0.5).is_none());
    }

    #[test]
    fn test_surface_query_with_heightfield() {
        let manifest = create_test_manifest_with_heightfield_surface();
        let query = SurfaceQuery::from_manifest(&manifest);

        assert_eq!(query.surface_count(), 1);

        // Query at known points on the heightfield
        let contact = query
            .ground_at(5.0, 5.0)
            .expect("heightfield test surface should resolve at an in-bounds point");
        let height = contact.height;
        assert!(
            height > 1.0 && height < 2.0,
            "Height {} should be between 1.0 and 2.0",
            height
        );

        // Query outside bounds
        let contact = query.ground_at(100.0, 100.0);
        assert!(contact.is_none());
    }

    #[test]
    fn test_mountain_switchback_route_height_increase() {
        // This test validates that the mountain switchback route shows continuous height increase
        // Based on the rolling_hills_test.world.json mountain_switchback_test data
        let manifest = create_test_manifest_with_heightfield_surface();
        let query = SurfaceQuery::from_manifest(&manifest);

        // Simulated mountain route points (adjusted for test heightfield bounds)
        let route_points = [
            (0.0, 0.0, 1.0),   // Base (minimum height of slope)
            (5.0, 5.0, 1.4),   // Middle-low
            (10.0, 10.0, 1.8), // Middle
            (15.0, 15.0, 2.2), // Middle-high
            (20.0, 20.0, 2.6), // Summit (maximum height of slope)
        ];

        let mut previous_height = 0.0;
        for (i, (x, z, expected_min_height)) in route_points.iter().enumerate() {
            if let Some(contact) = query.ground_at(*x, *z) {
                println!(
                    "Route point {}: ({}, {}) -> height {}",
                    i, x, z, contact.height
                );

                // Height should be at least the expected minimum
                assert!(
                    contact.height >= *expected_min_height,
                    "Route point {} height {} should be at least {}",
                    i,
                    contact.height,
                    expected_min_height
                );

                // For this route, height should generally increase (allowing for small variations)
                if i > 0 {
                    let height_increase = contact.height - previous_height;
                    // Height should increase or stay similar (within tolerance for terrain variations)
                    assert!(
                        height_increase >= -0.3,
                        "Route point {} should not have significant height drop. Previous: {}, Current: {}",
                        i, previous_height, contact.height
                    );
                }

                previous_height = contact.height;
            } else {
                panic!(
                    "Route point {} ({}, {}) should be on a walkable surface",
                    i, x, z
                );
            }
        }

        // Final height should be significantly higher than starting height
        let height_increase = previous_height - route_points[0].2;
        assert!(
            height_increase > 1.5,
            "Mountain route should show significant height increase. Start: {}, End: {}, Increase: {}",
            route_points[0].2, previous_height, height_increase
        );
    }

    #[test]
    fn test_plateau_access_route_height_increase() {
        // This test validates that the distant plateau access route shows continuous height increase
        // Based on the rolling_hills_test.world.json distant_plateau_test data
        let manifest = create_test_manifest_with_heightfield_surface();
        let query = SurfaceQuery::from_manifest(&manifest);

        // Simulated plateau access route points (adjusted for test heightfield bounds)
        let route_points = [
            (0.0, 0.0, 1.0),   // Start (minimum height of slope)
            (5.0, 5.0, 1.4),   // Middle-low
            (10.0, 10.0, 1.8), // Middle
            (15.0, 15.0, 2.2), // Middle-high
            (20.0, 20.0, 2.6), // Plateau top (maximum height of slope)
        ];

        let mut previous_height = 0.0;
        for (i, (x, z, expected_min_height)) in route_points.iter().enumerate() {
            if let Some(contact) = query.ground_at(*x, *z) {
                println!(
                    "Plateau route point {}: ({}, {}) -> height {}",
                    i, x, z, contact.height
                );

                // Height should be at least the expected minimum
                assert!(
                    contact.height >= *expected_min_height,
                    "Plateau route point {} height {} should be at least {}",
                    i,
                    contact.height,
                    expected_min_height
                );

                // Height should increase monotonically along this route
                if i > 0 {
                    assert!(
                        contact.height >= previous_height - 0.1,
                        "Plateau route point {} should not decrease in height. Previous: {}, Current: {}",
                        i, previous_height, contact.height
                    );
                }

                previous_height = contact.height;
            } else {
                panic!(
                    "Plateau route point {} ({}, {}) should be on a walkable surface",
                    i, x, z
                );
            }
        }

        // Final height should be higher than starting height
        let height_increase = previous_height - route_points[0].2;
        assert!(
            height_increase > 1.5,
            "Plateau route should show height increase. Start: {}, End: {}, Increase: {}",
            route_points[0].2,
            previous_height,
            height_increase
        );
    }

    #[test]
    fn test_backward_compatibility_v1_maps() {
        // This test validates backward compatibility with v1 maps (no surfaces)
        let v1_manifest = create_test_manifest_v1();
        let query = SurfaceQuery::from_manifest(&v1_manifest);

        // V1 maps should have empty surface queries
        assert!(query.is_empty());
        assert_eq!(query.surface_count(), 0);

        // All queries should return None (no walkable surfaces)
        assert!(query.ground_at(0.0, 0.0).is_none());
        assert!(query.ground_at(5.0, 5.0).is_none());
        assert!(query.ground_at(-5.0, -5.0).is_none());
    }

    // Helper functions for test fixtures

    fn create_test_manifest_v1() -> MapManifest {
        MapManifest {
            version: 1,
            map_id: "test_v1".to_string(),
            display_name: "Test V1".to_string(),
            bounds: MapBounds {
                min_x: -20.0,
                max_x: 20.0,
                min_z: -20.0,
                max_z: 20.0,
            },
            terrain: Default::default(),
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

    fn create_test_manifest_with_flat_surface() -> MapManifest {
        MapManifest {
            version: 2,
            map_id: "test_flat".to_string(),
            display_name: "Test Flat Surface".to_string(),
            bounds: MapBounds {
                min_x: -20.0,
                max_x: 20.0,
                min_z: -20.0,
                max_z: 20.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![WalkableSurface {
                id: "surface_test_flat".to_string(),
                kind: SurfaceKind::Flat,
                object: None,
                bounds: Some(SurfaceBounds {
                    min_x: 0.0,
                    max_x: 10.0,
                    min_z: 0.0,
                    max_z: 10.0,
                }),
                height: Some(1.5),
                min_height: None,
                max_height: None,
                grid_size: None,
                size: Some(10.0),
                purpose: Some("Test flat surface".to_string()),
                heightfield: None,
                walkable_mesh: None,
                layer: None,
                max_slope_deg: None,
            }],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        }
    }

    fn create_test_manifest_with_multiple_surfaces() -> MapManifest {
        MapManifest {
            version: 2,
            map_id: "test_multiple".to_string(),
            display_name: "Test Multiple Surfaces".to_string(),
            bounds: MapBounds {
                min_x: -20.0,
                max_x: 20.0,
                min_z: -20.0,
                max_z: 20.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![
                WalkableSurface {
                    id: "surface_1".to_string(),
                    kind: SurfaceKind::Flat,
                    object: None,
                    bounds: Some(SurfaceBounds {
                        min_x: 0.0,
                        max_x: 10.0,
                        min_z: 0.0,
                        max_z: 10.0,
                    }),
                    height: Some(1.5),
                    min_height: None,
                    max_height: None,
                    grid_size: None,
                    size: Some(10.0),
                    purpose: Some("First flat surface".to_string()),
                    heightfield: None,
                    walkable_mesh: None,
                    layer: None,
                    max_slope_deg: None,
                },
                WalkableSurface {
                    id: "surface_2".to_string(),
                    kind: SurfaceKind::Flat,
                    object: None,
                    bounds: Some(SurfaceBounds {
                        min_x: 10.0,
                        max_x: 20.0,
                        min_z: 10.0,
                        max_z: 20.0,
                    }),
                    height: Some(3.0),
                    min_height: None,
                    max_height: None,
                    grid_size: None,
                    size: Some(10.0),
                    purpose: Some("Second flat surface".to_string()),
                    heightfield: None,
                    walkable_mesh: None,
                    layer: None,
                    max_slope_deg: None,
                },
            ],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        }
    }

    fn create_test_manifest_with_heightfield_surface() -> MapManifest {
        let bounds = SurfaceBounds {
            min_x: 0.0,
            max_x: 20.0,
            min_z: 0.0,
            max_z: 20.0,
        };

        // Create a simple 4x4 heightfield (25 points)
        let mut heights = Vec::new();
        for x in 0..=4 {
            for z in 0..=4 {
                // Create a gentle slope from (0,0,1.0) to (20,20,2.6).
                // The route tests sample every 5m and expect a 0.4m climb per point.
                let xf = x as f32 / 4.0;
                let zf = z as f32 / 4.0;
                let height = 1.0 + (xf + zf) * 0.8;
                heights.push(height);
            }
        }

        let heightfield = HeightfieldData::new(4, bounds, heights);

        MapManifest {
            version: 2,
            map_id: "test_heightfield".to_string(),
            display_name: "Test Heightfield Surface".to_string(),
            bounds: MapBounds {
                min_x: -20.0,
                max_x: 20.0,
                min_z: -20.0,
                max_z: 20.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![WalkableSurface {
                id: "surface_heightfield".to_string(),
                kind: SurfaceKind::Mesh,
                object: None,
                bounds: None,
                height: None,
                min_height: Some(1.0),
                max_height: Some(2.6),
                grid_size: Some(4),
                size: Some(20.0),
                purpose: Some("Test heightfield surface".to_string()),
                heightfield: Some(heightfield),
                walkable_mesh: None,
                layer: None,
                max_slope_deg: None,
            }],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        }
    }

    /// Builds a manifest with two overlapping flat surfaces at different
    /// heights. Used to verify proximity-based surface selection.
    fn create_overlapping_surfaces_manifest() -> MapManifest {
        let bounds = SurfaceBounds {
            min_x: -10.0,
            max_x: 10.0,
            min_z: -10.0,
            max_z: 10.0,
        };
        MapManifest {
            version: CURRENT_VERSION,
            map_id: "overlapping".to_string(),
            display_name: "Overlapping".to_string(),
            bounds: MapBounds {
                min_x: -10.0,
                max_x: 10.0,
                min_z: -10.0,
                max_z: 10.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![
                WalkableSurface {
                    id: "surface_low".to_string(),
                    kind: SurfaceKind::Flat,
                    object: None,
                    bounds: Some(bounds),
                    height: Some(1.0),
                    min_height: None,
                    max_height: None,
                    grid_size: None,
                    size: None,
                    purpose: None,
                    heightfield: None,
                    walkable_mesh: None,
                    layer: None,
                    max_slope_deg: None,
                },
                WalkableSurface {
                    id: "surface_high".to_string(),
                    kind: SurfaceKind::Flat,
                    object: None,
                    bounds: Some(bounds),
                    height: Some(5.0),
                    min_height: None,
                    max_height: None,
                    grid_size: None,
                    size: None,
                    purpose: None,
                    heightfield: None,
                    walkable_mesh: None,
                    layer: None,
                    max_slope_deg: None,
                },
            ],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        }
    }

    #[test]
    fn test_ground_at_returns_highest_surface() {
        // ground_at keeps picking the highest overlapping surface (used by
        // raycasts / click-to-move target resolution).
        let query = SurfaceQuery::from_manifest(&create_overlapping_surfaces_manifest());
        let contact = query
            .ground_at(0.0, 0.0)
            .expect("overlapping surfaces should resolve");
        assert_eq!(contact.height, 5.0);
    }

    #[test]
    fn test_ground_at_reachable_asymmetric_step_limit() {
        // The reachability filter is asymmetric: it blocks climbing more than
        // max_step_height in a single tick (prevents cliff teleport up), but
        // always allows descending or snapping back down (so an entity is
        // never permanently stranded off-surface after spawn/teleport).
        let query = SurfaceQuery::from_manifest(&create_overlapping_surfaces_manifest());

        // From y=1.0 with a 0.45 step budget: the high surface (h=5.0) is
        // above the ceiling (1.0 + 0.45 = 1.45) so it is filtered out. Only
        // the low surface (h=1.0) is reachable.
        let contact = query
            .ground_at_reachable(0.0, 0.0, 1.0, 0.45)
            .expect("low surface should be reachable from itself");
        assert_eq!(contact.height, 1.0);

        // From y=5.0 with a 0.45 step budget: ceiling = 5.45, so BOTH surfaces
        // are below the ceiling. Highest-wins picks the high one.
        let contact = query
            .ground_at_reachable(0.0, 0.0, 5.0, 0.45)
            .expect("high surface should be reachable from itself");
        assert_eq!(contact.height, 5.0);

        // From y=10.0 (stranded well above both surfaces): the asymmetric
        // filter lets the entity snap back DOWN to the highest surface
        // instead of being permanently stuck.
        let contact = query
            .ground_at_reachable(0.0, 0.0, 10.0, 0.45)
            .expect("stranded entity should recover onto the highest surface below it");
        assert_eq!(contact.height, 5.0);
    }

    // ==================== TRIANGLE MESH TESTS ====================

    #[test]
    fn test_triangle_mesh_ramp() {
        // Create a simple 2-triangle ramp covering a 10×10 area
        // Triangle 1: (0,0,0), (10,0,0), (10,0,10) - right triangle with normal up
        // Triangle 2: (0,0,0), (10,0,10), (0,0,10) - left triangle with normal up
        let mesh = WalkableMeshData {
            vertices: vec![
                [0.0, 0.0, 0.0],   // v0
                [10.0, 0.0, 0.0],  // v1
                [10.0, 0.0, 10.0], // v2
                [0.0, 0.0, 10.0],  // v3
            ],
            indices: vec![0, 2, 1, 0, 3, 2], // Two triangles with correct winding
        };

        let surface = WalkableSurface {
            id: "test_ramp".to_string(),
            kind: SurfaceKind::Mesh,
            object: None,
            bounds: Some(SurfaceBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_z: 0.0,
                max_z: 10.0,
            }),
            height: None,
            min_height: Some(0.0),
            max_height: Some(5.0),
            grid_size: None,
            size: Some(10.0),
            purpose: Some("Test ramp".to_string()),
            heightfield: None,
            walkable_mesh: Some(mesh),
            layer: None,
            max_slope_deg: Some(45.0),
        };

        let manifest = MapManifest {
            version: 2,
            map_id: "test_ramp".to_string(),
            display_name: "Test Ramp".to_string(),
            bounds: MapBounds {
                min_x: -5.0,
                max_x: 15.0,
                min_z: -5.0,
                max_z: 15.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![surface],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let query = SurfaceQuery::from_manifest(&manifest);

        // Test point at center of area (5, 5) should have height 0.0 (flat triangles)
        let contact = query.ground_at(5.0, 5.0);
        assert!(contact.is_some());
        let ground = contact.expect("Ground contact should exist");
        assert_eq!(
            ground.height, 0.0,
            "Height should be 0.0 for flat triangles"
        );

        // Test point outside ramp should return None
        let contact = query.ground_at(15.0, 15.0);
        assert!(contact.is_none(), "Point outside ramp should return None");

        // Verify normal is flat (0,1,0) for horizontal surface
        let contact = query.ground_at(5.0, 5.0);
        assert!(contact.is_some());
        let ground = contact.expect("Ground contact should exist");
        assert_eq!(
            ground.normal,
            [0.0, 1.0, 0.0],
            "Normal should be flat for horizontal surface"
        );
    }

    #[test]
    fn test_triangle_mesh_crescent_hole() {
        // Create a frame shape with a hole in the center
        // This tests that the exact triangle test rejects points in the "hole"
        // even though they're within the 2D bounding box
        let mesh = WalkableMeshData {
            vertices: vec![
                // Outer frame vertices
                [0.0, 0.0, 0.0],   // v0
                [10.0, 0.0, 0.0],  // v1
                [10.0, 0.0, 10.0], // v2
                [0.0, 0.0, 10.0],  // v3
                // Inner hole vertices
                [2.0, 0.0, 2.0], // v4
                [8.0, 0.0, 2.0], // v5
                [8.0, 0.0, 8.0], // v6
                [2.0, 0.0, 8.0], // v7
            ],
            // Create 8 triangles for the frame with correct winding order
            // Each frame edge has two triangles
            indices: vec![
                0, 4, 5, // Bottom-left frame triangle
                0, 5, 1, // Bottom-right frame triangle
                1, 5, 6, // Right-bottom frame triangle
                1, 6, 2, // Right-top frame triangle
                2, 6, 7, // Top-right frame triangle
                2, 7, 3, // Top-left frame triangle
                3, 7, 4, // Left-top frame triangle
                3, 4, 0, // Left-bottom frame triangle
            ],
        };

        let surface = WalkableSurface {
            id: "test_crescent".to_string(),
            kind: SurfaceKind::Mesh,
            object: None,
            bounds: Some(SurfaceBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_z: 0.0,
                max_z: 10.0,
            }),
            height: None,
            min_height: Some(0.0),
            max_height: Some(0.0),
            grid_size: None,
            size: Some(10.0),
            purpose: Some("Crescent with hole".to_string()),
            heightfield: None,
            walkable_mesh: Some(mesh),
            layer: None,
            max_slope_deg: Some(45.0),
        };

        let manifest = MapManifest {
            version: 2,
            map_id: "test_crescent".to_string(),
            display_name: "Test Crescent".to_string(),
            bounds: MapBounds {
                min_x: -5.0,
                max_x: 15.0,
                min_z: -5.0,
                max_z: 15.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![surface],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let query = SurfaceQuery::from_manifest(&manifest);

        // Point in the center hole should return None (no triangle contains it)
        let contact = query.ground_at(5.0, 5.0);
        assert!(contact.is_none(), "Point in hole should return None");

        // Point on the outer ring should return Some height
        let contact = query.ground_at(1.0, 5.0);
        assert!(contact.is_some(), "Point on outer ring should return Some");

        // Point outside the mesh should return None
        let contact = query.ground_at(15.0, 15.0);
        assert!(contact.is_none(), "Point outside mesh should return None");
    }

    #[test]
    fn test_slope_rejection() {
        // Create a near-vertical triangle that should be rejected
        // Triangle with vertices at different heights to create steep slope
        let mesh = WalkableMeshData {
            vertices: vec![
                [0.0, 0.0, 0.0],  // v0 - ground level
                [1.0, 10.0, 0.0], // v1 - 10 units up, very steep
                [1.0, 0.0, 1.0],  // v2 - ground level
            ],
            indices: vec![0, 1, 2], // One steep triangle
        };

        let surface = WalkableSurface {
            id: "test_steep".to_string(),
            kind: SurfaceKind::Mesh,
            object: None,
            bounds: Some(SurfaceBounds {
                min_x: 0.0,
                max_x: 1.0,
                min_z: 0.0,
                max_z: 1.0,
            }),
            height: None,
            min_height: Some(0.0),
            max_height: Some(10.0),
            grid_size: None,
            size: Some(1.0),
            purpose: Some("Steep slope".to_string()),
            heightfield: None,
            walkable_mesh: Some(mesh),
            layer: None,
            max_slope_deg: Some(45.0), // Max 45 degree slope
        };

        let manifest = MapManifest {
            version: 2,
            map_id: "test_steep".to_string(),
            display_name: "Test Steep Slope".to_string(),
            bounds: MapBounds {
                min_x: -5.0,
                max_x: 5.0,
                min_z: -5.0,
                max_z: 5.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![surface],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let query = SurfaceQuery::from_manifest(&manifest);

        // The steep triangle should be rejected, returning None
        let contact = query.ground_at(0.5, 0.5);
        assert!(contact.is_none(), "Steep triangle should be rejected");
    }

    #[test]
    fn test_normal_computation() {
        // Create a tilted triangle and verify normal is computed correctly
        // Copy the passing ramp test structure exactly, just with one vertex raised
        let mesh = WalkableMeshData {
            vertices: vec![
                [0.0, 0.0, 0.0],   // v0
                [10.0, 0.0, 0.0],  // v1
                [10.0, 0.0, 10.0], // v2 - will be raised
                [0.0, 0.0, 10.0],  // v3
            ],
            // Same indices as ramp test: 0, 2, 1, 0, 3, 2
            indices: vec![0, 2, 1, 0, 3, 2],
        };

        // Manually raise v2 to create a tilted surface
        let mut vertices = mesh.vertices.clone();
        vertices[2] = [10.0, 3.0, 10.0]; // Raise v2 by 3 units
        let tilted_mesh = WalkableMeshData {
            vertices,
            indices: mesh.indices,
        };

        let surface = WalkableSurface {
            id: "test_normal".to_string(),
            kind: SurfaceKind::Mesh,
            object: None,
            bounds: Some(SurfaceBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_z: 0.0,
                max_z: 10.0,
            }),
            height: None,
            min_height: Some(0.0),
            max_height: Some(10.0),
            grid_size: None,
            size: Some(10.0),
            purpose: Some("Normal test".to_string()),
            heightfield: None,
            walkable_mesh: Some(tilted_mesh),
            layer: None,
            max_slope_deg: Some(45.0),
        };

        let manifest = MapManifest {
            version: 2,
            map_id: "test_normal".to_string(),
            display_name: "Test Normal".to_string(),
            bounds: MapBounds {
                min_x: -5.0,
                max_x: 15.0,
                min_z: -5.0,
                max_z: 15.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![surface],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let query = SurfaceQuery::from_manifest(&manifest);

        // Test point that should be on the tilted surface
        let contact = query.ground_at(8.0, 2.0);
        assert!(contact.is_some());
        let ground = contact.expect("Ground contact should exist");

        // Normal should not be flat (0,1,0) for tilted surface
        assert_ne!(
            ground.normal,
            [0.0, 1.0, 0.0],
            "Normal should not be flat for tilted surface"
        );

        // Normal should be normalized (length ≈ 1.0)
        let normal_length =
            (ground.normal[0].powi(2) + ground.normal[1].powi(2) + ground.normal[2].powi(2)).sqrt();
        assert!(
            (normal_length - 1.0).abs() < 0.01,
            "Normal should be normalized"
        );

        // Y component should be positive (pointing up)
        assert!(
            ground.normal[1] > 0.0,
            "Normal should point up (positive Y)"
        );
    }

    // ==================== BLOCKER COLLISION TESTS ====================

    #[test]
    fn test_blocker_collision() {
        use super::super::shapes::CollisionShape;
        use crate::manifest::{BlockerData, BlockerKind, TransformData};

        // Create a blocker at position (5, 0, 5) with cylinder shape
        let blocker = BlockerData {
            id: "test_blocker".to_string(),
            kind: BlockerKind::Cylinder,
            object: Some("test_blocker".to_string()),
            transform: Some(TransformData {
                translation: [5.0, 0.0, 5.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            }),
            shape: Some(CollisionShape::Cylinder {
                radius: 2.0,
                height: 3.0,
            }),
            blocks_movement: true,
        };

        let manifest = MapManifest {
            version: 2,
            map_id: "test_blocker".to_string(),
            display_name: "Test Blocker".to_string(),
            bounds: MapBounds {
                min_x: 0.0,
                max_x: 20.0,
                min_z: 0.0,
                max_z: 20.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![],
            traversals: vec![],
            blockers: vec![blocker],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let grid = CollisionGrid::build(&manifest);

        // Point near blocker should be blocked
        assert!(
            grid.is_blocked([5.0, 0.0, 5.0], 0.5),
            "Point near blocker should be blocked"
        );

        // Point far from blocker should not be blocked
        assert!(
            !grid.is_blocked([15.0, 0.0, 15.0], 0.5),
            "Point far from blocker should not be blocked"
        );

        // Point exactly at blocker edge should be blocked
        assert!(
            grid.is_blocked([7.0, 0.0, 5.0], 0.1),
            "Point at blocker edge should be blocked"
        );

        // Point just outside blocker should not be blocked
        assert!(
            !grid.is_blocked([8.0, 0.0, 5.0], 0.5),
            "Point just outside blocker should not be blocked"
        );
    }

    #[test]
    fn a_distant_blocker_does_not_block_the_origin() {
        use super::super::shapes::CollisionShape;
        use crate::manifest::{BlockerData, BlockerKind, TransformData};

        let near = BlockerData {
            id: "near".to_string(),
            kind: BlockerKind::Box,
            object: None,
            transform: Some(TransformData::at(0.0, 1.0, 0.0)),
            shape: Some(CollisionShape::Box {
                half_extents: [0.5, 1.0, 0.5],
            }),
            blocks_movement: true,
        };
        let far = BlockerData {
            id: "far".to_string(),
            kind: BlockerKind::Box,
            object: None,
            transform: Some(TransformData::at(80.0, 1.0, 80.0)),
            shape: Some(CollisionShape::Box {
                half_extents: [0.5, 1.0, 0.5],
            }),
            blocks_movement: true,
        };
        let manifest = MapManifest {
            version: 2,
            map_id: "hash".to_string(),
            display_name: "Hash".to_string(),
            bounds: MapBounds {
                min_x: -10.0,
                max_x: 90.0,
                min_z: -10.0,
                max_z: 90.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![],
            traversals: vec![],
            blockers: vec![near, far],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };
        let grid = CollisionGrid::build(&manifest);
        assert!(grid.is_blocked([0.0, 0.0, 0.0], 0.4));
        assert!(!grid.is_blocked([40.0, 0.0, 40.0], 0.4));
        assert!(grid.is_blocked([80.0, 0.0, 80.0], 0.4));
    }

    /// Regression for the Y-aware `is_blocked` fix.
    ///
    /// A blocking obstacle placed on the ground must not stop an entity
    /// standing on a platform directly above it. The obstacle's top
    /// (`max[1]`) is well below the player Y minus the tolerance, so the
    /// reject branch kicks in and the query returns false.
    #[test]
    fn test_blocker_below_player_does_not_block() {
        use super::super::shapes::CollisionShape;
        use crate::manifest::{BlockerData, BlockerKind, TransformData};

        // Cylinder at y=0, height=1.0 → AABB Y range [-0.5, +0.5].
        let blocker = BlockerData {
            id: "rock_under_bridge".to_string(),
            kind: BlockerKind::Cylinder,
            object: Some("rock_under_bridge".to_string()),
            transform: Some(TransformData {
                translation: [5.0, 0.0, 5.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            }),
            shape: Some(CollisionShape::Cylinder {
                radius: 2.0,
                height: 1.0,
            }),
            blocks_movement: true,
        };

        let manifest = MapManifest {
            version: 2,
            map_id: "stacked_layers".to_string(),
            display_name: "Stacked Layers".to_string(),
            bounds: MapBounds {
                min_x: 0.0,
                max_x: 20.0,
                min_z: 0.0,
                max_z: 20.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![],
            traversals: vec![],
            blockers: vec![blocker],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let grid = CollisionGrid::build(&manifest);

        // Same-floor player still gets blocked (player_y at obstacle top).
        assert!(
            grid.is_blocked([5.0, 0.5, 5.0], 0.5),
            "Player at the rock's top should still collide"
        );

        // Player on a raised platform 5m above should be ignored.
        assert!(
            !grid.is_blocked([5.0, 5.0, 5.0], 0.5),
            "Rock on the ground must not block a player on a platform above"
        );
    }

    #[test]
    fn test_blocker_without_transform_or_shape_skipped() {
        use super::super::shapes::CollisionShape;
        use crate::manifest::{BlockerData, BlockerKind, TransformData};

        // Create a blocker without transform (should be skipped)
        let blocker_no_transform = BlockerData {
            id: "test_blocker".to_string(),
            kind: BlockerKind::Cylinder,
            object: Some("test_blocker".to_string()),
            transform: None, // Missing transform
            shape: Some(CollisionShape::Cylinder {
                radius: 2.0,
                height: 3.0,
            }),
            blocks_movement: true,
        };

        // Create a blocker without shape (should be skipped)
        let blocker_no_shape = BlockerData {
            id: "test_blocker2".to_string(),
            kind: BlockerKind::Cylinder,
            object: Some("test_blocker2".to_string()),
            transform: Some(TransformData {
                translation: [10.0, 0.0, 10.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            }),
            shape: None, // Missing shape
            blocks_movement: true,
        };

        // Create a blocker with blocks_movement = false (should be skipped)
        let blocker_non_blocking = BlockerData {
            id: "test_blocker3".to_string(),
            kind: BlockerKind::Cylinder,
            object: Some("test_blocker3".to_string()),
            transform: Some(TransformData {
                translation: [15.0, 0.0, 15.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            }),
            shape: Some(CollisionShape::Cylinder {
                radius: 2.0,
                height: 3.0,
            }),
            blocks_movement: false, // Not blocking
        };

        let manifest = MapManifest {
            version: 2,
            map_id: "test_blocker_skip".to_string(),
            display_name: "Test Blocker Skip".to_string(),
            bounds: MapBounds {
                min_x: 0.0,
                max_x: 20.0,
                min_z: 0.0,
                max_z: 20.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: None,
            surfaces: vec![],
            traversals: vec![],
            blockers: vec![blocker_no_transform, blocker_no_shape, blocker_non_blocking],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let grid = CollisionGrid::build(&manifest);

        // All blockers should have been skipped, so no obstacles
        assert_eq!(grid.obstacle_count(), 0, "All blockers should be skipped");

        // No point should be blocked since there are no obstacles
        assert!(
            !grid.is_blocked([5.0, 0.0, 5.0], 0.5),
            "No blocking should occur"
        );
        assert!(
            !grid.is_blocked([10.0, 0.0, 10.0], 0.5),
            "No blocking should occur"
        );
        assert!(
            !grid.is_blocked([15.0, 0.0, 15.0], 0.5),
            "No blocking should occur"
        );
    }

    #[test]
    fn test_surface_contact_at_resolves_steep_mountain_slope() {
        // Build a surface with a steep slope (> 45 degrees, e.g. 60 degrees)
        let bounds = SurfaceBounds {
            min_x: 0.0,
            max_x: 10.0,
            min_z: 0.0,
            max_z: 10.0,
        };
        // 2x2 heightfield: rises from 0.0 to 20.0 over 10 units in X (slope > 60 deg)
        let heights = vec![0.0, 0.0, 20.0, 20.0];
        let hf = HeightfieldData::new(1, bounds, heights);
        let manifest = MapManifest {
            version: 2,
            map_id: "steep_mountain".to_string(),
            display_name: "Steep Mountain".to_string(),
            bounds: MapBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_z: 0.0,
                max_z: 10.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics {
                max_walkable_slope_deg: 45.0,
                ..Default::default()
            }),
            surfaces: vec![WalkableSurface {
                id: "steep_cliff".to_string(),
                kind: SurfaceKind::Mesh,
                object: None,
                bounds: Some(bounds),
                height: None,
                min_height: Some(0.0),
                max_height: Some(20.0),
                grid_size: None,
                size: None,
                purpose: None,
                heightfield: Some(hf),
                walkable_mesh: None,
                layer: None,
                max_slope_deg: Some(45.0),
            }],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };

        let query = SurfaceQuery::from_manifest(&manifest);

        // At x = 5.0, height should be 10.0.
        // ground_at must return None because slope exceeds 45 degrees (not walkable).
        assert!(
            query.ground_at(5.0, 5.0).is_none(),
            "ground_at should reject steep slopes > 45 deg"
        );

        // surface_contact_at must return Some with height 10.0 for aiming/raycasting.
        let contact = query.surface_contact_at(5.0, 5.0);
        assert!(
            contact.is_some(),
            "surface_contact_at should resolve steep mountain height"
        );
        let contact = contact.unwrap();
        assert!(
            (contact.height - 10.0).abs() < 1e-3,
            "Height should be 10.0 on the slope"
        );
    }
}
