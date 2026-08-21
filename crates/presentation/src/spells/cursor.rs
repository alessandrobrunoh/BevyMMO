//! Dove punta il mouse, tradotto in un punto del terreno.
//!
//! Fornisce il punto di mira unificato per l'input delle abilità e
//! l'anteprima di mira (aim preview), che lo richiede ogni frame
//! invece che al solo istante del click.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevymmo_client::movement::{resolve_ray_to_ground, ClientSurfaceQuery};

/// Punto in cui il raggio della camera sotto il cursore incontra la superficie
/// del terreno (o il piano `y = 0` se non ci sono superfici caricate).
///
/// `None` se non c'è finestra, cursore fuori dalla finestra, nessuna camera 3D
/// o raggio che non interseca il terreno.
pub fn cursor_ground_point(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &Transform), With<Camera3d>>,
    surface_query: Option<&ClientSurfaceQuery>,
) -> Option<Vec3> {
    let cursor_position = windows.single().ok()?.cursor_position()?;
    let (camera, camera_transform) = cameras.iter().next()?;
    let view = crate::renderer::camera_view(camera_transform);
    let ray = camera.viewport_to_world(&view, cursor_position).ok()?;
    surface_query
        .and_then(|sq| sq.0.as_ref())
        .and_then(|sq| resolve_ray_to_ground(ray.origin, *ray.direction, sq, 300.0, 0.5))
        .or_else(|| bevymmo_client::movement::intersect_y0_plane(ray.origin, *ray.direction, 300.0))
}

/// Direzione orizzontale normalizzata da `origin` verso `target`, o `None` se
/// i due punti coincidono (nel qual caso il facing corrente va lasciato stare).
pub fn flat_direction_towards(origin: Vec3, target: Vec3) -> Option<Vec3> {
    let offset = Vec3::new(target.x - origin.x, 0.0, target.z - origin.z);
    let length = offset.length();
    (length > 0.001).then(|| offset / length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_direction_towards_computes_horizontal_normalized_offset() {
        let origin = Vec3::new(1.0, 10.0, 1.0);
        let target = Vec3::new(4.0, 20.0, 5.0);
        let dir = flat_direction_towards(origin, target).expect("should compute direction");
        assert!((dir.y).abs() < f32::EPSILON);
        assert!((dir.length() - 1.0).abs() < 1e-4);
        assert!((dir.x - 0.6).abs() < 1e-4);
        assert!((dir.z - 0.8).abs() < 1e-4);
    }

    #[test]
    fn flat_direction_towards_returns_none_for_coincident_points() {
        let origin = Vec3::new(5.0, 10.0, 5.0);
        let target = Vec3::new(5.0, 25.0, 5.0);
        assert_eq!(flat_direction_towards(origin, target), None);
    }
}
