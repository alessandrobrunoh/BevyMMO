use bevy::color::Color;
use bevy::prelude::*;

/// Genera un nodo di sfondo e un nodo di riempimento a posizionamento
/// assoluto per la barra, agganciati a `parent`.
/// Ritorna `(bar_entity, fill_entity)`.
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
                // Centra eventuali figli (come il testo) che potrebbero
                // essere aggiunti a questa barra.
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::BLACK),
            BackgroundColor(bg_color),
        ))
        .id();

    commands.entity(parent).add_child(bar_entity);

    // Nodo di riempimento
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
