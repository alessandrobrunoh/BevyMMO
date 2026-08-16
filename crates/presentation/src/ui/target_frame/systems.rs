//! Sistemi per il target frame (UI panel con info sul target selezionato).

use crate::ui::bar::spawn_bar;
use crate::ui::target_frame::components::{TargetFrame, TargetFrameParts, TargetFrameTarget};
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;
use bevy::prelude::*;
use bevymmo_gameplay::entity::components::{EntityKind, PlayerName};
use bevymmo_network::network::protocol::Position;
use bevymmo_gameplay::stats::components::VitalStats;
use bevymmo_client::targeting::CurrentTarget;

const FRAME_WIDTH: f32 = 200.0;
const HP_BAR_HEIGHT: f32 = 12.0;
const PADDING: f32 = 8.0;
const ROW_GAP: f32 = 4.0;

/// Spawna il target frame UI per il target specificato.
pub fn spawn_target_frame(
    commands: &mut Commands,
    target_entity: Entity,
    theme: &UiTheme,
) -> Entity {
    let container = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(100.0),
                width: Val::Px(FRAME_WIDTH),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(PADDING)),
                row_gap: Val::Px(ROW_GAP),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            TargetFrame,
            TargetFrameTarget {
                entity: target_entity,
            },
        ))
        .id();

    // Backdrop per il nome
    let name_backdrop = commands
        .spawn(Node {
            padding: UiRect::all(Val::Px(2.0)),
            ..default()
        })
        .id();
    commands.entity(container).add_child(name_backdrop);

    // Nome del target
    let name_text = spawn_text(commands, name_backdrop, "Target", 16.0, theme.text_color);

    // Tipo di entità
    let kind_text = spawn_text(commands, container, "Unknown", 12.0, theme.text_color);

    // HP bar
    let (bar, fill) = spawn_bar(
        commands,
        container,
        1.0,
        1.0,
        Vec2::new(FRAME_WIDTH - PADDING * 2.0, HP_BAR_HEIGHT),
        theme.bar_bg,
        theme.hp_fill,
    );

    // HP text
    let hp_text = spawn_text(commands, bar, "?/?", 12.0, theme.text_color);

    commands
        .entity(container)
        .insert(TargetFrameParts::new(name_text, hp_text, kind_text, fill));

    container
}

/// Sistema: gestisce la visibilità e la creazione del target frame in base al target corrente.
pub fn manage_target_frame(
    mut commands: Commands,
    current_target: Res<CurrentTarget>,
    theme: Res<UiTheme>,
    target_query: Query<(
        &Position,
        &VitalStats,
        Option<&PlayerName>,
        Option<&EntityKind>,
    )>,
    frame_query: Query<(Entity, &TargetFrameTarget)>,
) {
    let current_target_entity = current_target.entity;

    // Check if we have a target
    let target_entity = match current_target_entity {
        Some(entity) => entity,
        None => {
            // No target: remove any existing frame
            for (frame_entity, ..) in frame_query.iter() {
                commands.entity(frame_entity).despawn();
            }
            return;
        }
    };

    // Try to get the target's components
    let (_target_position, target_vital, _target_name, _target_kind) =
        match target_query.get(target_entity) {
            Ok((pos, vital, name, kind)) => (pos, vital, name, kind),
            Err(_) => {
                // Target doesn't exist or doesn't have required components: remove any existing frame
                for (frame_entity, ..) in frame_query.iter() {
                    commands.entity(frame_entity).despawn();
                }
                return;
            }
        };

    // Check if target is dead
    if target_vital.is_dead() {
        for (frame_entity, ..) in frame_query.iter() {
            commands.entity(frame_entity).despawn();
        }
        return;
    }

    // Check if we already have a frame for this target
    let mut existing_frame_for_target = None;
    for (frame_entity, frame_target) in frame_query.iter() {
        if frame_target.entity == target_entity {
            existing_frame_for_target = Some(frame_entity);
            break;
        }
    }

    match existing_frame_for_target {
        Some(_) => {
            // Frame exists: will be updated by update_target_frame_content
        }
        None => {
            // No frame for this target: remove any old frames and create a new one
            for (frame_entity, ..) in frame_query.iter() {
                commands.entity(frame_entity).despawn();
            }
            spawn_target_frame(&mut commands, target_entity, &theme);
        }
    }
}

/// Sistema: aggiorna il contenuto del target frame (nome, HP, tipo).
pub fn update_target_frame_content(
    target_query: Query<(&VitalStats, Option<&PlayerName>, Option<&EntityKind>)>,
    theme: Res<UiTheme>,
    mut frame_query: Query<(&TargetFrameTarget, &mut TargetFrameParts)>,
    mut text_query: Query<&mut Text>,
    mut node_query: Query<&mut Node>,
    mut bg_query: Query<&mut BackgroundColor>,
) {
    for (frame_target, mut parts) in frame_query.iter_mut() {
        let Ok((vital, name, entity_kind)) = target_query.get(frame_target.entity) else {
            continue;
        };

        // Nome: scrittura + cache solo se il valore è cambiato
        let new_name = name
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "Entity".to_string());
        if parts.last_name != new_name {
            if let Ok(mut text) = text_query.get_mut(parts.name_text) {
                text.0 = new_name.clone();
            }
            parts.last_name = new_name;
        }

        // HP text: "current/max" intero
        let new_hp_text = format!(
            "{}/{}",
            vital.current_health as i32, vital.max_health as i32
        );
        if parts.last_hp_text != new_hp_text {
            if let Ok(mut text) = text_query.get_mut(parts.hp_text) {
                text.0 = new_hp_text.clone();
            }
            parts.last_hp_text = new_hp_text;
        }

        // Entity kind text
        let kind_name = entity_kind.map_or_else(
            || "Unknown".to_string(),
            |k| match k {
                EntityKind::Player => "Player".to_string(),
                EntityKind::Friendly => "Friendly".to_string(),
                EntityKind::Neutral => "Neutral".to_string(),
                EntityKind::Hostile => "Hostile".to_string(),
            },
        );
        if parts.last_kind_text != kind_name {
            if let Ok(mut text) = text_query.get_mut(parts.kind_text) {
                text.0 = kind_name.clone();
            }
            parts.last_kind_text = kind_name;
        }

        // HP fill percentage
        let new_hp_pct = (vital.current_health / vital.max_health.max(0.1)).clamp(0.0, 1.0) * 100.0;
        if parts.last_hp_pct != new_hp_pct {
            if let Ok(mut fill_node) = node_query.get_mut(parts.hp_fill) {
                fill_node.width = Val::Percent(new_hp_pct);
            }
            parts.last_hp_pct = new_hp_pct;
        }

        // HP fill color based on EntityKind
        let new_fill_color = get_hp_fill_color(entity_kind, &theme);
        if let Ok(mut bg) = bg_query.get_mut(parts.hp_fill) {
            if bg.0 != new_fill_color {
                bg.0 = new_fill_color;
            }
        }
    }
}

/// Sistema: rimuove i frame quando il target non è più valido.
pub fn cleanup_target_frames(
    mut commands: Commands,
    target_query: Query<&VitalStats>,
    frame_query: Query<(Entity, &TargetFrameTarget)>,
) {
    for (frame_entity, frame_target) in frame_query.iter() {
        if let Ok(vital) = target_query.get(frame_target.entity) {
            // Rimuovi frame se il target è morto
            if vital.is_dead() {
                commands.entity(frame_entity).despawn();
            }
        } else {
            // Rimuovi frame se il target non esiste o non ha più VitalStats
            commands.entity(frame_entity).despawn();
        }
    }
}

/// Determina il colore della barra HP in base al tipo di entità.
fn get_hp_fill_color(entity_kind: Option<&EntityKind>, theme: &UiTheme) -> Color {
    match entity_kind {
        Some(EntityKind::Player) => Color::srgb(0.3, 0.8, 0.5),
        Some(EntityKind::Friendly) => Color::srgb(0.2, 0.9, 0.3),
        Some(EntityKind::Neutral) => Color::srgb(0.9, 0.9, 0.2),
        Some(EntityKind::Hostile) => Color::srgb(0.9, 0.1, 0.1),
        None => theme.hp_fill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_is_hidden_when_target_is_cleared() {
        let mut app = App::new();
        app.init_resource::<UiTheme>();
        app.add_systems(Update, manage_target_frame);

        // Setup: target con tutti i componenti
        let target = app
            .world_mut()
            .spawn((
                Position(Vec3::new(10.0, 0.0, 5.0)),
                VitalStats {
                    current_health: 100.0,
                    max_health: 100.0,
                    max_mana: 40.0,
                    mana_regeneration: 2.0,
                },
                PlayerName("TestTarget".to_string()),
                EntityKind::Hostile,
            ))
            .id();

        // Setup: imposta il target corrente
        app.world_mut().insert_resource(CurrentTarget::new(target));

        app.update();

        // Verifica: il frame dovrebbe esistere
        let frame_count = app
            .world_mut()
            .query::<&TargetFrame>()
            .iter(app.world())
            .count();
        assert_eq!(frame_count, 1);

        // Clear target
        app.world_mut().resource_mut::<CurrentTarget>().clear();

        app.update();

        // Verifica: il frame dovrebbe essere stato rimosso
        let frame_count_after = app
            .world_mut()
            .query::<&TargetFrame>()
            .iter(app.world())
            .count();
        assert_eq!(frame_count_after, 0);
    }

    #[test]
    fn frame_updates_when_target_health_changes() {
        let mut app = App::new();
        app.init_resource::<UiTheme>();
        app.add_systems(
            Update,
            (manage_target_frame, update_target_frame_content).chain(),
        );

        // Setup: target
        let target = app
            .world_mut()
            .spawn((
                Position(Vec3::ZERO),
                VitalStats {
                    current_health: 100.0,
                    max_health: 100.0,
                    max_mana: 40.0,
                    mana_regeneration: 2.0,
                },
                PlayerName("TestTarget".to_string()),
                EntityKind::Hostile,
            ))
            .id();

        app.world_mut().insert_resource(CurrentTarget::new(target));

        app.update();

        // Cambia HP del target
        app.world_mut()
            .entity_mut(target)
            .get_mut::<VitalStats>()
            .unwrap()
            .current_health = 50.0;

        app.update();

        // Verifica: il testo HP dovrebbe essere aggiornato
        let mut parts_query = app.world_mut().query::<&TargetFrameParts>();
        let parts = parts_query.single(app.world()).unwrap();
        assert_eq!(parts.last_hp_text, "50/100");
    }

    #[test]
    fn frame_is_removed_when_target_dies() {
        let mut app = App::new();
        app.init_resource::<UiTheme>();
        app.add_systems(Update, manage_target_frame);

        // Setup: target vivo
        let target = app
            .world_mut()
            .spawn((
                Position(Vec3::ZERO),
                VitalStats {
                    current_health: 100.0,
                    max_health: 100.0,
                    max_mana: 40.0,
                    mana_regeneration: 2.0,
                },
                EntityKind::Hostile,
            ))
            .id();

        app.world_mut().insert_resource(CurrentTarget::new(target));

        app.update();

        // Verifica: il frame dovrebbe esistere
        let frame_count = app
            .world_mut()
            .query::<&TargetFrame>()
            .iter(app.world())
            .count();
        assert_eq!(frame_count, 1);

        // Uccidi il target
        app.world_mut()
            .entity_mut(target)
            .get_mut::<VitalStats>()
            .unwrap()
            .current_health = 0.0;

        app.update();

        // Verifica: il frame dovrebbe essere stato rimosso
        let frame_count_after = app
            .world_mut()
            .query::<&TargetFrame>()
            .iter(app.world())
            .count();
        assert_eq!(frame_count_after, 0);
    }

    #[test]
    fn hp_fill_color_is_red_for_hostile() {
        let color = get_hp_fill_color(Some(&EntityKind::Hostile), &UiTheme::default());
        assert_eq!(color, Color::srgb(0.9, 0.1, 0.1));
    }

    #[test]
    fn hp_fill_color_is_green_for_player() {
        let color = get_hp_fill_color(Some(&EntityKind::Player), &UiTheme::default());
        assert_eq!(color, Color::srgb(0.3, 0.8, 0.5));
    }

    #[test]
    fn hp_fill_color_is_yellow_for_neutral() {
        let color = get_hp_fill_color(Some(&EntityKind::Neutral), &UiTheme::default());
        assert_eq!(color, Color::srgb(0.9, 0.9, 0.2));
    }

    #[test]
    fn hp_fill_color_is_green_for_friendly() {
        let color = get_hp_fill_color(Some(&EntityKind::Friendly), &UiTheme::default());
        assert_eq!(color, Color::srgb(0.2, 0.9, 0.3));
    }
}
