//! Reusable UI button.
//!
//! The button pairs a semantic action ([`UiButtonAction`]) with an
//! interactive node; the `action -> effect` mapping lives in the central UI systems
//! (`crate::ui::systems::update_button_actions`) and not in the component.

use bevy::prelude::*;

use crate::ui::theme::UiTheme;

/// Effect triggered by pressing the button.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiButtonAction {
    Play,
    OpenSettings,
    BackToMenu,
    Resume,
    ReturnToMainMenu,
    Exit,
    /// Settings → Keybinds → "Reset to defaults".
    ResetKeybinds,
}

/// UI Button with associated semantic action.
#[derive(Component)]
pub struct UiButton {
    pub action: UiButtonAction,
}

/// Spawns a button with label and theme-consistent style, attached to `parent`.
///
/// Returns the button entity (useful for testing or future references).
pub fn spawn_button(
    commands: &mut Commands,
    parent: Entity,
    label: impl Into<String>,
    action: UiButtonAction,
    theme: &UiTheme,
) -> Entity {
    let button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(44.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(theme.button_bg),
            UiButton { action },
        ))
        .id();

    commands.entity(parent).add_child(button);

    let label_entity = commands
        .spawn((
            Text::new(label.into()),
            TextFont {
                font_size: FontSize::Px(theme.button_font_size),
                ..default()
            },
            TextColor(theme.button_text_color),
        ))
        .id();
    commands.entity(button).add_child(label_entity);

    button
}
