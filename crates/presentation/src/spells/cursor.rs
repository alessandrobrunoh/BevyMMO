//! Dove punta il mouse, tradotto in un punto del terreno.
//!
//! Fornisce il punto di mira unificato per l'input delle abilità e
//! l'anteprima di mira (aim preview), che lo richiede ogni frame
//! invece che al solo istante del click.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Punto in cui il raggio della camera sotto il cursore incontra il piano
/// `y = 0`, con la Y azzerata.
///
/// `None` se non c'è finestra, cursore fuori dalla finestra, nessuna camera 3D
/// o raggio parallelo al piano.
///
/// Nota: il piano è il livello del mare, non il terreno vero (che il movimento
/// campiona invece con `bevymmo_shared::movement::resolve_ray_to_ground`), per
/// cui la mira resta imprecisa in pendenza — comportamento invariato rispetto
/// alle tre copie che questa funzione sostituisce.
pub fn cursor_ground_point(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Option<Vec3> {
    let cursor_position = windows.single().ok()?.cursor_position()?;
    let (camera, camera_transform) = cameras.iter().next()?;
    let ray = camera
        .viewport_to_world(camera_transform, cursor_position)
        .ok()?;
    let point = ray.plane_intersection_point(
        Vec3::ZERO,
        bevy::math::primitives::InfinitePlane3d::new(Vec3::Y),
    )?;
    Some(Vec3::new(point.x, 0.0, point.z))
}

/// Direzione orizzontale normalizzata da `origin` verso `target`, o `None` se
/// i due punti coincidono (nel qual caso il facing corrente va lasciato stare).
pub fn flat_direction_towards(origin: Vec3, target: Vec3) -> Option<Vec3> {
    let offset = Vec3::new(target.x - origin.x, 0.0, target.z - origin.z);
    let length = offset.length();
    (length > 0.001).then(|| offset / length)
}
