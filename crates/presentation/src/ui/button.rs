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
    Logout,
    Exit,
    /// Settings → Keybinds → "Reset to defaults".
    ResetKeybinds,
}

/// UI Button with associated semantic action.
#[derive(Component)]
pub struct UiButton {
    pub action: UiButtonAction,
}

const BUTTON_DEFAULT_PATH: &str = "buttons/button-default.png";
const BUTTON_HOVER_PATH: &str = "buttons/button-hover.png";
const BUTTON_CLICKED_PATH: &str = "buttons/button-clicked.png";

/// Textures used by the three button interaction states.
#[derive(Component, Clone)]
pub struct UiButtonImages {
    pub default: Handle<Image>,
    pub hover: Handle<Image>,
    pub clicked: Handle<Image>,
}

/// Spawns a button with separate textures for default, hover, and clicked, attached to `parent`.
///
/// Returns the button entity (useful for testing or future references).
pub fn spawn_button(
    commands: &mut Commands,
    parent: Entity,
    label: impl Into<String>,
    action: UiButtonAction,
    theme: &UiTheme,
    asset_server: &AssetServer,
) -> Entity {
    let images = UiButtonImages {
        default: asset_server.load(BUTTON_DEFAULT_PATH),
        hover: asset_server.load(BUTTON_HOVER_PATH),
        clicked: asset_server.load(BUTTON_CLICKED_PATH),
    };

    let button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(48.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode::new(images.default.clone()),
            UiButtonImages { ..images },
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
