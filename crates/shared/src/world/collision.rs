//! Shared, lightweight world collision.
//!
//! This is intentionally an AABB broad-phase for the first playable map slice:
//! no physics engine, no mesh queries, and identical math on server/client.

use super::manifest::MapManifest;
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
            let Some(shape) = prop.collision else { continue };
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
