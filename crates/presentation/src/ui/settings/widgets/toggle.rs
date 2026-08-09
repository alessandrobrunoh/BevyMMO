//! Toggle (checkbox) widget.

use bevy::ecs::component::Component;
use bevy::prelude::*;

use crate::ui::theme::UiTheme;

/// A labeled on/off toggle.
#[derive(Component, Clone)]
pub struct Toggle {
    /// Stable identifier used by the caller to dispatch the change.
    pub id: String,
    pub on: bool,
}

#[derive(Component)]
pub struct ToggleLabel;

/// Visual representation of the toggle state (checkbox square).
#[derive(Component)]
pub struct ToggleDisplay;

/// Checkbox control used by settings panels.
pub type CheckBox = Toggle;

/// Spawns a labeled toggle row and returns its root button entity.
pub fn spawn_toggle(
    commands: &mut Commands,
    parent: Entity,
    id: impl Into<String>,
    label: impl Into<String>,
    on: bool,
    theme: &UiTheme,
) -> Entity {
    let button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(420.0),
                height: Val::Px(44.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(theme.input_bg),
            BorderColor::all(theme.input_border),
            Toggle { id: id.into(), on },
        ))
        .id();
    commands.entity(parent).add_child(button);

    let label_entity = commands
        .spawn((
            Text::new(label.into()),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size),
                ..default()
            },
            TextColor(theme.text_color),
            ToggleLabel,
        ))
        .id();
    commands.entity(button).add_child(label_entity);

    let display_entity = commands
        .spawn((
            Node {
                width: Val::Px(20.0),
                height: Val::Px(20.0),
                margin: UiRect {
                    left: Val::Auto,
                    ..default()
                },
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(if on {
                theme.button_hovered_bg
            } else {
                Color::NONE
            }),
            BorderColor::all(theme.input_border_focused),
            ToggleDisplay,
        ))
        .id();
    commands.entity(button).add_child(display_entity);

    button
}

/// Spawns a [`CheckBox`] control.
pub fn spawn_checkbox(
    commands: &mut Commands,
    parent: Entity,
    id: impl Into<String>,
    label: impl Into<String>,
    on: bool,
    theme: &UiTheme,
) -> Entity {
    spawn_toggle(commands, parent, id, label, on, theme)
}
