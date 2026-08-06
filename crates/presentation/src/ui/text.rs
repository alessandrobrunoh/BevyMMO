use bevy::prelude::*;

/// Genera un semplice nodo di testo UI e lo aggancia a `parent`.
pub fn spawn_text(
    commands: &mut Commands,
    parent: Entity,
    text: impl Into<String>,
    font_size: f32,
    color: Color,
) -> Entity {
    let child = commands
        .spawn((
            Text::new(text.into()),
            TextFont {
                font_size: bevy::prelude::FontSize::Px(font_size),
                ..default()
            },
            TextColor(color),
            Node::default(),
        ))
        .id();

    commands.entity(parent).add_child(child);
    child
}
