//! Sistemi per la sidebar NPC: raycast click, selezione target più vicino,
//! spawn/despawn Card UI.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevymmo_client::stdb::{commands, StdbConnection};
use bevymmo_gameplay::entity::components::{EntityKind, GameEntity, PlayerName};
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_network::network::protocol::Position;
use bevymmo_network::world_components::NetworkEntityId;

use crate::ui::card::components::CardPositioning;
use crate::ui::card::{CardBuilder, CardKind};
use crate::ui::npc_sidebar::components::{NpcSidebar, VendorItemButton};
use crate::ui::theme::UiTheme;

/// Distanza massima (in unità di mondo) dal raggio del cursore entro cui un
/// NPC può essere selezionato.
///
/// Senza questa soglia, `closest_friendly_hit` sceglie sempre l'NPC più
/// vicino alla retta del raggio, non importa quanto lontano dal cursore
/// stesso — un click ovunque sullo schermo aprirebbe la sidebar. Stesso
/// raggio usato da `select_target_with_left_click` per coerenza visiva tra
/// le due selezioni.
const NPC_SELECT_RADIUS: f32 = 1.2;

/// Rappresenta un potenziale hit di un'entità durante il raycast.
///
/// Usato dal helper puro [`closest_friendly_hit`] per selezionare il target
/// più vicino alla camera/origine del raggio.
#[derive(Debug, Clone, Copy)]
pub struct EntityHit {
    pub entity: Entity,
    pub distance: f32,
}

/// Seleziona l'hit più vicino da una lista di candidati.
///
/// Funzione pura testabile: restituisce l'`EntityHit` con la distanza minima,
/// o `None` se la lista è vuota.
pub fn closest_friendly_hit(hits: &[EntityHit]) -> Option<Entity> {
    hits.iter()
        .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal))
        .map(|hit| hit.entity)
}

/// Sistema principale: al click sinistro, raycast dalla Camera3d e trova il
/// NPC (EntityKind::Friendly) più vicino. Se colpito, despawna la sidebar
/// precedente e ne crea una nuova.
#[allow(clippy::too_many_arguments)]
pub fn npc_sidebar_on_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    theme: Res<UiTheme>,
    item_registry: Res<ItemRegistry>,
    // Query per le entità game (NPC = GameEntity + Position + EntityKind::Friendly)
    entity_query: Query<(Entity, &Position, &EntityKind), With<GameEntity>>,
    name_query: Query<&PlayerName>,
    // Query per le sidebar esistenti
    existing_sidebar: Query<Entity, With<NpcSidebar>>,
) {
    // Solo al frame del click sinistro
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    // Ottieni raggio dalla camera attraverso il cursore
    let Some(ray) = cursor_ray(&windows, &cameras) else {
        return;
    };

    // Costruisci lista di hits per le entità Friendly
    let mut hits: Vec<EntityHit> = Vec::new();
    for (entity, position, kind) in entity_query.iter() {
        if *kind != EntityKind::Friendly {
            continue;
        }

        // Distanza approssimativa: distanza dal punto più vicino sul raggio
        // all'entità (proiezione del punto sull'asse del raggio)
        let distance = point_to_ray_distance(position.0, ray.origin, *ray.direction);
        if distance > NPC_SELECT_RADIUS {
            continue;
        }
        hits.push(EntityHit { entity, distance });
    }

    // Trova il più vicino
    let Some(target_entity) = closest_friendly_hit(&hits) else {
        return;
    };

    // Despawna sidebar precedente
    for sidebar_entity in existing_sidebar.iter() {
        commands.entity(sidebar_entity).despawn();
    }

    // Ottieni nome del NPC (fallback "NPC")
    let npc_name = name_query
        .get(target_entity)
        .map(|name| name.0.clone())
        .unwrap_or_else(|_| "NPC".to_string());

    // Spawn nuova Card
    spawn_npc_sidebar(&mut commands, &theme, target_entity, &npc_name, &item_registry);
}

/// Calcola la distanza tra un punto e un raggio (linea infinita).
///
/// Restituisce la distanza perpendicolare dal punto al raggio, non la
/// distanza lungo il raggio. Questo favorisce i NPC vicini alla linea di mira.
fn point_to_ray_distance(point: Vec3, ray_origin: Vec3, ray_direction: Vec3) -> f32 {
    let to_point = point - ray_origin;
    let projection = to_point.dot(ray_direction);
    let closest_on_ray = ray_origin + ray_direction * projection.clamp(0.0, f32::MAX);
    point.distance(closest_on_ray)
}

/// Ottiene il raggio dalla Camera3d attraverso il cursore nella PrimaryWindow.
fn cursor_ray(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Option<Ray3d> {
    let window = windows.single().ok()?;
    let cursor_pos = window.cursor_position()?;
    let (camera, transform) = cameras.iter().next()?;
    camera.viewport_to_world(transform, cursor_pos).ok()
}

/// Spawna una Card UI per la sidebar NPC.
fn spawn_npc_sidebar(
    commands: &mut Commands,
    theme: &UiTheme,
    target_entity: Entity,
    npc_name: &str,
    item_registry: &ItemRegistry,
) {
    let card_entity = CardBuilder::new(CardKind::Generic, npc_name)
        .width(Val::Px(320.0))
        .height(Val::Px(360.0))
        .positioning(CardPositioning::Left)
        .closeable()
        .exclusive()
        .with_body(|body| {
            body.spawn((
                Text::new("Ciao! Scegli un oggetto:"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(theme.text_color),
            ));
            for (_, item) in item_registry.sorted_items().into_iter()
                .filter(|(_, item)| item.config().equippable_into.is_some())
            {
                body.spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(30.0),
                        margin: UiRect::vertical(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(theme.button_bg),
                    VendorItemButton {
                        npc: target_entity,
                        item_id: item.id(),
                    },
                )).with_children(|button| {
                    button.spawn((
                        Text::new(item.display_name().to_string()),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size),
                            ..default()
                        },
                        TextColor(theme.text_color),
                    ));
                });
            }
        })
        .spawn(commands, theme);

    // Aggiunge il marker NpcSidebar alla root della Card
    commands.entity(card_entity).insert(NpcSidebar {
        target: target_entity,
    });
}

/// Sends the server-authoritative claim request for a clicked vendor item.
/// The reducer rechecks NPC type and proximity, so a stale sidebar cannot grant
/// an item after the player has moved away.
pub fn claim_vendor_item(
    interactions: Query<(&Interaction, &VendorItemButton), Changed<Interaction>>,
    npc_entities: Query<&NetworkEntityId>,
    connection: Option<Res<StdbConnection>>,
) {
    let Some(connection) = connection else {
        return;
    };
    for (interaction, button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Ok(network_id) = npc_entities.get(button.npc) else {
            continue;
        };
        if let Err(error) = commands::claim_npc_item(
            &connection,
            network_id.0,
            button.item_id.as_str().to_string(),
        ) {
            error!("could not claim NPC item: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closest_friendly_hit_returns_none_for_empty_list() {
        let hits: Vec<EntityHit> = vec![];
        assert_eq!(closest_friendly_hit(&hits), None);
    }

    #[test]
    fn closest_friendly_hit_returns_single_entity() {
        let hits = vec![EntityHit {
            entity: Entity::PLACEHOLDER,
            distance: 5.0,
        }];
        assert_eq!(
            closest_friendly_hit(&hits),
            Some(Entity::PLACEHOLDER)
        );
    }

    #[test]
    fn closest_friendly_hit_returns_closest_entity() {
        let far = Entity::from_raw_u32(1).expect("valid entity index");
        let close = Entity::from_raw_u32(2).expect("valid entity index");
        let hits = vec![
            EntityHit { entity: far, distance: 100.0 },
            EntityHit { entity: close, distance: 10.0 },
        ];
        assert_eq!(closest_friendly_hit(&hits), Some(close));
    }

    #[test]
    fn closest_friendly_hit_handles_equal_distances() {
        let a = Entity::from_raw_u32(1).expect("valid entity index");
        let b = Entity::from_raw_u32(2).expect("valid entity index");
        let hits = vec![
            EntityHit { entity: a, distance: 50.0 },
            EntityHit { entity: b, distance: 50.0 },
        ];
        // Qualsiasi dei due è accettabile; verifichiamo solo che sia uno dei due
        let result = closest_friendly_hit(&hits);
        assert!(result == Some(a) || result == Some(b));
    }

    #[test]
    fn point_to_ray_distance_is_zero_for_point_on_ray() {
        let origin = Vec3::ZERO;
        let direction = Vec3::Z;
        let point_on_ray = Vec3::new(0.0, 0.0, 5.0); // Sull'asse Z dall'origine
        let distance = point_to_ray_distance(point_on_ray, origin, direction);
        assert!((distance).abs() < f32::EPSILON);
    }

    #[test]
    fn point_to_ray_distance_is_positive_for_off_axis() {
        let origin = Vec3::ZERO;
        let direction = Vec3::Z;
        let off_axis = Vec3::new(10.0, 0.0, 5.0); // 10 unità a destra del raggio
        let distance = point_to_ray_distance(off_axis, origin, direction);
        assert!((distance - 10.0).abs() < 1e-4);
    }
}
