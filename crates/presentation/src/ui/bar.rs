use bevy::color::Color;
use bevy::prelude::*;
use bevymmo_gameplay::entity::components::EntityKind;

use crate::ui::theme::UiTheme;

/// Fill color for an HP bar, keyed by the entity's disposition towards the
/// local player. `None` (disposition unknown/not yet replicated) falls back
/// to the theme's neutral HP color instead of guessing a disposition.
pub fn get_hp_fill_color(entity_kind: Option<&EntityKind>, theme: &UiTheme) -> Color {
    match entity_kind {
        Some(EntityKind::Player) => Color::srgb(0.3, 0.8, 0.5),
        Some(EntityKind::Friendly) => Color::srgb(0.2, 0.9, 0.3),
        Some(EntityKind::Neutral) => Color::srgb(0.9, 0.9, 0.2),
        Some(EntityKind::Hostile) => Color::srgb(0.9, 0.1, 0.1),
        None => theme.hp_fill,
    }
}

/// Spawns a background node and an absolutely positioned fill node
/// for the bar, attached to `parent`.
/// Returns `(bar_entity, fill_entity)`.
pub fn spawn_bar(
    commands: &mut Commands,
    parent: Entity,
    current_val: f32,
    max_val: f32,
    size: Vec2,
    bg_color: Color,
    fill_color: Color,
) -> (Entity, Entity) {
    let percentage = (current_val / max_val.max(0.1)).clamp(0.0, 1.0);

    let bar_entity = commands
        .spawn((
            Node {
                width: Val::Px(size.x),
                height: Val::Px(size.y),
                border: UiRect::all(Val::Px(1.0)),
                // Centers any children (like text) that might
                // be added to this bar.
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::BLACK),
            BackgroundColor(bg_color),
        ))
        .id();

    commands.entity(parent).add_child(bar_entity);

    // Fill node
    let fill = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(percentage * 100.0),
                ..default()
            },
            BackgroundColor(fill_color),
        ))
        .id();

    commands.entity(bar_entity).add_child(fill);

    (bar_entity, fill)
}

/// Finds the singleton full-screen absolute root marked `M`, or spawns one.
///
/// Several floating-bar overlays (entity HP bars, crowd control bars, ...)
/// each need exactly one full-screen `Node` to parent their per-target
/// widgets to, distinguished only by which marker component tags it.
pub fn get_or_spawn_root<M: Component + Default>(
    commands: &mut Commands,
    query: &Query<Entity, With<M>>,
) -> Entity {
    if let Ok(entity) = query.single() {
        return entity;
    }

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            M::default(),
        ))
        .id()
}
