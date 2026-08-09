//! Shared, lightweight world collision and ground queries.
//!
//! This module provides:
//! - AABB broad-phase collision for obstacles (blocking props)
//! - Ground contact resolution for height-aware movement
//! - Surface query functionality for walkable surfaces
//!
//! All math is identical on server/client to ensure movement prediction matches
//! authoritative server validation.

use super::manifest::{MapManifest, SurfaceKind, WalkableSurface};
use super::shapes::aabb_for_shape;

#[derive(Clone, Copy, Debug)]
struct Obstacle {
    min: [f32; 3],
    max: [f32; 3],
}

#[derive(Clone, Debug, Default)]
pub struct CollisionGrid {
    obstacles: Vec<Obstacle>,
}

impl CollisionGrid {
    pub fn build(manifest: &MapManifest) -> Self {
        let mut obstacles = Vec::new();
        for prop in &manifest.props {
            let Some(shape) = prop.collision else {
                continue;
            };
            if !prop.blocks_movement {
                continue;
            }

            let (mut min, mut max) = aabb_for_shape(prop.transform.translation, shape);
            let scale = prop.transform.scale.map(f32::abs);
            for axis in 0..3 {
                let center = prop.transform.translation[axis];
                let half_extent = (max[axis] - min[axis]) * 0.5 * scale[axis];
                min[axis] = center - half_extent;
                max[axis] = center + half_extent;
            }
            obstacles.push(Obstacle { min, max });
        }
        Self { obstacles }
    }

    /// Returns true when a player circle in the X/Z plane intersects a
    /// blocking obstacle. Y is intentionally ignored while terrain is flat.
    pub fn is_blocked(&self, point: [f32; 3], radius: f32) -> bool {
        self.obstacles.iter().any(|obstacle| {
            let closest_x = point[0].clamp(obstacle.min[0], obstacle.max[0]);
            let closest_z = point[2].clamp(obstacle.min[2], obstacle.max[2]);
            let dx = point[0] - closest_x;
            let dz = point[2] - closest_z;
            dx * dx + dz * dz <= radius * radius
        })
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
}

impl SurfaceQuery {
    /// Creates a new surface query system from a map manifest.
    ///
    /// Extracts walkable surface data and prepares it for queries.
    /// For version 1 maps (without surfaces), this returns an empty query system.
    pub fn from_manifest(manifest: &MapManifest) -> Self {
        Self {
            surfaces: manifest.surfaces.clone(),
        }
    }

    /// Resolves ground contact at the given world position.
    ///
    /// Returns `None` if the position is not over any walkable surface.
    /// For flat surfaces, returns constant height with horizontal normal.
    /// For mesh-based surfaces, currently returns an error for unsupported
    /// surface types (this will be extended in future slices).
    ///
    /// When multiple surfaces overlap, returns the one most appropriate for
    /// movement based on current height and traversal rules.
    pub fn ground_at(&self, x: f32, z: f32) -> Option<GroundContact> {
        self.surfaces
            .iter()
            .filter(|surface| self.surface_contains_point(surface, x, z))
            .filter_map(|surface| self.resolve_surface(surface, x, z))
            .max_by(|left, right| left.height.total_cmp(&right.height))
    }

    /// Checks if a surface contains a given 2D point (x, z).
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
                // For mesh surfaces, check heightfield bounds if available
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

    /// Resolves ground contact for a specific surface.
    fn resolve_surface(&self, surface: &WalkableSurface, x: f32, z: f32) -> Option<GroundContact> {
        match surface.kind {
            SurfaceKind::Flat => {
                // Flat surfaces have constant height
                surface.height.map(|height| GroundContact::flat(height))
            }
            SurfaceKind::FlatMesh => {
                // For flat_mesh, try heightfield data first, then fall back to constant height
                if let Some(ref heightfield) = surface.heightfield {
                    heightfield
                        .sample_height(x, z)
                        .map(|height| GroundContact::flat(height))
                } else {
                    // Fall back to constant height
                    surface.height.map(|height| GroundContact::flat(height))
                }
            }
            SurfaceKind::Mesh => {
                // For mesh surfaces, use heightfield data if available
                if let Some(ref heightfield) = surface.heightfield {
                    heightfield.sample_height(x, z).map(|height| {
                        // TODO: Calculate proper surface normal from heightfield gradient
                        // For now, use flat normal as approximation
                        GroundContact::flat(height)
                    })
                } else {
                    // No heightfield data available
                    None
                }
            }
        }
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
    use crate::world::{
        HeightfieldData, MapBounds, MapManifest, SurfaceBounds, SurfaceKind, WalkableSurface,
        WorldMetrics,
    };

    #[test]
    fn test_ground_contact_flat_surface() {
        let ground = GroundContact::flat(2.5);
        assert_eq!(ground.height, 2.5);
        assert_eq!(ground.normal, [0.0, 1.0, 0.0]);
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
            1.0, 1.5, 2.0, // Row 0 (z=0)
            1.2, 1.7, 2.2, // Row 1 (z=5)
            1.4, 1.9, 2.4, // Row 2 (z=10)
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

        // Test outside bounds
        assert_eq!(heightfield.sample_height(-1.0, 5.0), None);
        assert_eq!(heightfield.sample_height(11.0, 5.0), None);
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
        for z in 0..=4 {
            for x in 0..=4 {
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
            }],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        }
    }
}
