//! Toggle (switch) widget.

use bevy::ecs::component::Component;
use bevy::prelude::*;

use crate::ui::settings::state::SettingToggle;
use crate::ui::theme::UiTheme;

/// A labeled on/off toggle.
#[derive(Component, Clone)]
pub struct Toggle {
    /// Stable identifier used by the caller to dispatch the change.
    pub id: SettingToggle,
    pub on: bool,
}

#[derive(Component)]
pub struct ToggleLabel;

/// Track of the switch. Fill colour follows on/off.
#[derive(Component)]
pub struct ToggleDisplay;

/// Knob that sits on the left (off) or right (on) of the track.
#[derive(Component)]
pub struct ToggleKnob;

/// Checkbox control used by settings panels.
pub type CheckBox = Toggle;

const TRACK_WIDTH: f32 = 48.0;
const TRACK_HEIGHT: f32 = 24.0;
const KNOB_SIZE: f32 = 18.0;
const KNOB_INSET: f32 = 3.0;

fn track_color(on: bool, theme: &UiTheme) -> Color {
    if on {
        theme.button_hovered_bg
    } else {
        Color::srgb(0.16, 0.16, 0.2)
    }
}

fn knob_margin(on: bool) -> UiRect {
    if on {
        UiRect {
            left: Val::Auto,
            right: Val::Px(KNOB_INSET),
            top: Val::Px(KNOB_INSET),
            bottom: Val::Px(KNOB_INSET),
        }
    } else {
        UiRect {
            left: Val::Px(KNOB_INSET),
            right: Val::Auto,
            top: Val::Px(KNOB_INSET),
            bottom: Val::Px(KNOB_INSET),
        }
    }
}

/// Applies on/off colours and knob position. Shared by spawn and click.
pub fn apply_toggle_visual(
    on: bool,
    theme: &UiTheme,
    track: &mut BackgroundColor,
    knob: &mut Node,
) {
    track.0 = track_color(on, theme);
    knob.margin = knob_margin(on);
}

/// Spawns a labeled switch row and returns its root button entity.
pub fn spawn_toggle(
    commands: &mut Commands,
    parent: Entity,
    id: SettingToggle,
    label: impl Into<String>,
    on: bool,
    theme: &UiTheme,
) -> Entity {
    let button = commands
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(44.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(theme.input_bg),
            BorderColor::all(theme.input_border),
            Toggle { id, on },
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

    let track = commands
        .spawn((
            Node {
                width: Val::Px(TRACK_WIDTH),
                height: Val::Px(TRACK_HEIGHT),
                margin: UiRect {
                    left: Val::Auto,
                    ..default()
                },
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(TRACK_HEIGHT / 2.0)),
                ..default()
            },
            BackgroundColor(track_color(on, theme)),
            BorderColor::all(theme.input_border_focused),
            ToggleDisplay,
        ))
        .id();
    commands.entity(button).add_child(track);

    let knob = commands
        .spawn((
            Node {
                width: Val::Px(KNOB_SIZE),
                height: Val::Px(KNOB_SIZE),
                margin: knob_margin(on),
                border_radius: BorderRadius::all(Val::Percent(50.0)),
                ..default()
            },
            BackgroundColor(theme.text_color),
            ToggleKnob,
        ))
        .id();
    commands.entity(track).add_child(knob);

    button
}

/// Spawns a [`CheckBox`] control.
pub fn spawn_checkbox(
    commands: &mut Commands,
    parent: Entity,
    id: SettingToggle,
    label: impl Into<String>,
    on: bool,
    theme: &UiTheme,
) -> Entity {
    spawn_toggle(commands, parent, id, label, on, theme)
}
