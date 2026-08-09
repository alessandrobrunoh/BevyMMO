//! Dropdown / select widget.
//!
//! Spawns a labeled button whose click cycles to the next item in the list.
//! The current value is shown next to the label.

use bevy::ecs::component::Component;
use bevy::ecs::message::Message;
use bevy::prelude::*;

use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

/// One selectable option in the dropdown.
#[derive(Clone, Debug)]
pub struct DropdownItem {
    /// Display string, shown to the user.
    pub label: String,
    /// Opaque string identifier stored in the widget. The caller decodes it.
    pub value: String,
}

/// A dropdown widget: label + value, click cycles to the next item.
///
/// The list of items lives in the component so the cycle system can move the
/// cursor forward without external lookups.
#[derive(Component, Clone)]
pub struct Dropdown {
    /// Stable identifier used by the caller to dispatch the change.
    pub id: String,
    pub items: Vec<DropdownItem>,
    /// Index of the currently selected item.
    pub selected: usize,
}

/// Marker: text node showing the current value next to the label.
#[derive(Component)]
pub struct DropdownValueText;

/// Event emitted when the dropdown selection changes (click or programmatic).
#[derive(Message, Clone, Debug)]
pub struct DropdownChanged {
    pub id: String,
    pub value: String,
}

/// Spawns a labeled dropdown and returns its root entity.
///
/// The dropdown is laid out as a row button:
/// `[ label ............ value ▾ ]`.
pub fn spawn_dropdown(
    commands: &mut Commands,
    parent: Entity,
    id: impl Into<String>,
    label: impl Into<String>,
    items: Vec<DropdownItem>,
    initial_value: &str,
    theme: &UiTheme,
) -> Entity {
    let id = id.into();
    let selected = items
        .iter()
        .position(|i| i.value == initial_value)
        .unwrap_or(0);

    let button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(420.0),
                height: Val::Px(44.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(theme.input_bg),
            BorderColor::all(theme.input_border),
            Dropdown {
                id,
                items,
                selected,
            },
        ))
        .id();
    commands.entity(parent).add_child(button);

    let label_entity = spawn_text(
        commands,
        button,
        label.into(),
        theme.input_font_size,
        theme.text_color,
    );

    let value_entity = commands
        .spawn((
            Text::new(String::new()),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size),
                ..default()
            },
            TextColor(theme.muted_text_color),
            DropdownValueText,
        ))
        .id();
    commands.entity(button).add_child(label_entity);
    commands.entity(button).add_child(value_entity);

    button
}
