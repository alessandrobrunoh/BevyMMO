//! Key-capture widget: shows the current binding and listens for a new key
//! when clicked.
//!
//! Capture state lives in the component itself. The system
//! [`crate::ui::settings::systems::update_key_capture_input`] reads raw
//! keyboard events for widgets in capture mode and mutates the widget, then
//! a separate event ([`KeyBindingChanged`]) propagates the change to the
//! settings resource.

use bevy::ecs::component::Component;
use bevy::ecs::message::Message;
use bevy::prelude::*;

use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

use super::super::state::{KeyAction, KeyBinding};

/// A key-capture widget.
#[derive(Component, Clone)]
pub struct KeyCapture {
    /// Action being rebound.
    pub action: KeyAction,
    /// Current binding shown when not capturing.
    pub binding: KeyBinding,
    /// True while waiting for the next key press.
    pub capturing: bool,
}

#[derive(Component)]
pub struct KeyCaptureLabel;

#[derive(Component)]
pub struct KeyCaptureDisplay;

/// Event emitted when the user finishes capturing a new binding.
#[derive(Message, Clone, Debug)]
pub struct KeyBindingChanged {
    pub action: KeyAction,
    pub binding: KeyBinding,
}

/// Spawns a key-capture row (label + button showing current binding) and
/// returns its root entity.
pub fn spawn_key_capture(
    commands: &mut Commands,
    parent: Entity,
    action: KeyAction,
    binding: KeyBinding,
    theme: &UiTheme,
) -> Entity {
    let row = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(44.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);

    let label_entity = commands
        .spawn((
            Text::new(action.label().to_string()),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size),
                ..default()
            },
            TextColor(theme.text_color),
            KeyCaptureLabel,
        ))
        .id();
    commands.entity(row).add_child(label_entity);

    let button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(160.0),
                height: Val::Px(36.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(theme.input_bg),
            BorderColor::all(theme.input_border),
            KeyCapture {
                action,
                binding,
                capturing: false,
            },
            KeyCaptureDisplay,
        ))
        .id();
    commands.entity(row).add_child(button);

    let value_text = spawn_text(
        commands,
        button,
        binding.label(),
        theme.input_font_size,
        theme.text_color,
    );
    let _ = value_text;

    row
}
