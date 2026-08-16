//! Systems for right-click targeting and automatic cleanup.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use bevymmo_shared::entity::components::GameEntity;
use bevymmo_shared::network::protocol::Position;
use bevymmo_shared::stats::components::VitalStats;
use bevymmo_shared::targeting::CurrentTarget;

const TARGETING_RADIUS: f32 = 1.2;

/// Ray-sphere intersection test.
///
/// Calculates if a ray intersects a sphere and returns the distance from ray origin
/// to the nearest intersection point.
///
/// # Arguments
/// * `ray_origin` - ray origin point
/// * `ray_direction` - normalized ray direction
/// * `sphere_center` - sphere center
/// * `sphere_radius` - sphere radius
///
/// # Returns
/// `Some(distance)` if ray intersects sphere, `None` otherwise.
/// Distance is always positive.
fn ray_sphere_intersection(
    ray_origin: Vec3,
    ray_direction: Vec3,
    sphere_center: Vec3,
    sphere_radius: f32,
) -> Option<f32> {
    let oc = ray_origin - sphere_center;
    let a = ray_direction.dot(ray_direction);
    let b = 2.0 * oc.dot(ray_direction);
    let c = oc.dot(oc) - sphere_radius * sphere_radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    // Solve for smallest root (closest intersection)
    let sqrt_discriminant = discriminant.sqrt();
    let t1 = (-b - sqrt_discriminant) / (2.0 * a);
    let t2 = (-b + sqrt_discriminant) / (2.0 * a);

    // Return smallest positive, or None if both are negative
    if t1 >= 0.0 && t2 >= 0.0 {
        Some(t1.min(t2))
    } else if t1 >= 0.0 {
        Some(t1)
    } else if t2 >= 0.0 {
        Some(t2)
    } else {
        None
    }
}

/// Target selection system with right click.
///
/// Executes the following steps:
/// 1. Reads cursor position
/// 2. Casts a ray from Camera3d
/// 3. Finds all targetable entities (GameEntity + Position + VitalStats)
/// 4. Filters dead entities
/// 5. Ray-sphere test with each entity
/// 6. Selects closest entity along ray
pub fn select_target_with_right_click(
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut current_target: ResMut<CurrentTarget>,
    targetable_entities: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
) {
    let Some(mouse_buttons) = mouse_buttons else {
        return;
    };

    // Left click only, do not affect right-click movement
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    let mut closest_hit: Option<(Entity, f32)> = None;

    for (entity, position, vital_stats) in targetable_entities.iter() {
        // Ignore dead entities
        if vital_stats.is_dead() {
            continue;
        }

        if let Some(distance) =
            ray_sphere_intersection(ray.origin, *ray.direction, position.0, TARGETING_RADIUS)
        {
            match closest_hit {
                None => {
                    closest_hit = Some((entity, distance));
                }
                Some((_, current_dist)) if distance < current_dist => {
                    closest_hit = Some((entity, distance));
                }
                _ => {}
            }
        }
    }

    if let Some((entity, _)) = closest_hit {
        current_target.set(entity);
    } else {
        current_target.clear();
    }
}

/// System to clear target with the configured "Clear Target" key.
pub fn clear_target_with_escape(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    settings: Option<Res<bevymmo_shared::user_settings::GameSettingsResource>>,
    mut current_target: ResMut<CurrentTarget>,
) {
    let Some(keyboard) = keyboard else {
        return;
    };
    let Some(settings) = settings else {
        return;
    };

    if settings.just_pressed(bevymmo_shared::user_settings::KeyAction::ClearTarget, &keyboard) {
        current_target.clear();
    }
}

/// Automatic target cleanup system.
///
/// Periodically checks if the current target is still valid:
/// - Entity still exists
/// - Still has Position and VitalStats components
/// - Is not dead
///
/// If any condition fails, target is cleared.
pub fn cleanup_invalid_target(
    mut current_target: ResMut<CurrentTarget>,
    targetable_entities: Query<(&Position, &VitalStats), With<GameEntity>>,
) {
    let Some(target_entity) = current_target.entity else {
        return;
    };

    // Verify entity exists and has required components
    let Ok((_position, vital_stats)) = targetable_entities.get(target_entity) else {
        current_target.clear();
        return;
    };

    // Verify entity is not dead
    if vital_stats.is_dead() {
        current_target.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_sphere_hits_directly() {
        let origin = Vec3::ZERO;
        let direction = Vec3::X;
        let center = Vec3::new(5.0, 0.0, 0.0);
        let radius = 1.0;

        let hit = ray_sphere_intersection(origin, direction, center, radius);
        assert!(hit.is_some());
        // Approximate distance: 5.0 - 1.0 = 4.0
        assert!(hit.unwrap() > 3.9 && hit.unwrap() < 4.1);
    }

    #[test]
    fn ray_sphere_misses() {
        let origin = Vec3::ZERO;
        let direction = Vec3::X;
        let center = Vec3::new(5.0, 5.0, 0.0);
        let radius = 1.0;

        let hit = ray_sphere_intersection(origin, direction, center, radius);
        assert!(hit.is_none());
    }

    #[test]
    fn ray_sphere_origin_inside() {
        let origin = Vec3::ZERO;
        let direction = Vec3::X;
        let center = Vec3::new(0.5, 0.0, 0.0);
        let radius = 2.0;

        let hit = ray_sphere_intersection(origin, direction, center, radius);
        assert!(hit.is_some());
        // Should be positive and relatively small
        assert!(hit.unwrap() >= 0.0);
    }

    #[test]
    fn ray_sphere_misses_backward() {
        let origin = Vec3::ZERO;
        let direction = -Vec3::X;
        let center = Vec3::new(5.0, 0.0, 0.0);
        let radius = 1.0;

        let hit = ray_sphere_intersection(origin, direction, center, radius);
        assert!(hit.is_none());
    }
}

