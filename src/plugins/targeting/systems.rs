//! Sistemi per il targeting con tasto destro e pulizia automatica.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::network::protocol::Position;
use crate::plugins::entity::components::GameEntity;
use crate::plugins::targeting::resources::CurrentTarget;
use crate::stats::components::VitalStats;

const TARGETING_RADIUS: f32 = 1.2;

/// Test di intersezione ray-sphere.
///
/// Calcola se un ray interseca una sfera e restituisce la distanza dal ray origin
/// al punto di intersezione più vicino.
///
/// # Argomenti
/// * `ray_origin` - punto di origine del ray
/// * `ray_direction` - direzione normalizzata del ray
/// * `sphere_center` - centro della sfera
/// * `sphere_radius` - raggio della sfera
///
/// # Restituisce
/// `Some(distance)` se il ray interseca la sfera, `None` altrimenti.
/// La distanza è sempre positiva.
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

    // Risolviamo per la radice più piccola (intersezione più vicina)
    let sqrt_discriminant = discriminant.sqrt();
    let t1 = (-b - sqrt_discriminant) / (2.0 * a);
    let t2 = (-b + sqrt_discriminant) / (2.0 * a);

    // Restituiamo la più piccola positiva, o None se entrambe sono negative
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
        // Distanza approssimativa: 5.0 - 1.0 = 4.0
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
        // Dovrebbe essere positivo e relativamente piccolo
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

/// Sistema di selezione target con tasto destro.
///
/// Esegue i seguenti passaggi:
/// 1. Legge la posizione del cursore
/// 2. Casta un ray dalla Camera3d
/// 3. Trova tutte le entità targettabili (GameEntity + Position + VitalStats)
/// 4. Filtra le entità morte
/// 5. Test ray-sphere con ogni entità
/// 6. Seleziona l'entità più vicina lungo il ray
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

    // Solo tasto destro, non influenziamo il movimento (sinistro)
    if !mouse_buttons.just_pressed(MouseButton::Right) {
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
        // Ignora entità morte
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

/// Sistema per pulire il target con il tasto Escape.
pub fn clear_target_with_escape(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut current_target: ResMut<CurrentTarget>,
) {
    let Some(keyboard) = keyboard else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Escape) {
        current_target.clear();
    }
}

/// Sistema di pulizia automatica del target.
///
/// Controlla periodicamente che il target corrente sia ancora valido:
/// - L'entità esiste ancora
/// - Ha ancora i componenti Position e VitalStats
/// - Non è morta
///
/// Se una di queste condizioni fallisce, il target viene pulito.
pub fn cleanup_invalid_target(
    mut current_target: ResMut<CurrentTarget>,
    targetable_entities: Query<(&Position, &VitalStats), With<GameEntity>>,
) {
    let Some(target_entity) = current_target.entity else {
        return;
    };

    // Verifica che l'entità esista e abbia i componenti richiesti
    let Ok((_position, vital_stats)) = targetable_entities.get(target_entity) else {
        current_target.clear();
        return;
    };

    // Verifica che l'entità non sia morta
    if vital_stats.is_dead() {
        current_target.clear();
    }
}
