//! Pulsante UI riusabile.
//!
//! Il pulsante accoppia un'azione semantica ([`UiButtonAction`]) a un nodo
//! interattivo; la mappatura `azione -> effetto` vive nei sistemi UI centrali
//! (`crate::ui::systems::update_button_actions`) e non nel componente.

use bevy::prelude::*;

use crate::ui::theme::UiTheme;

/// Effetto scatenato dalla pressione del pulsante.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiButtonAction {
    Play,
    OpenSettings,
    BackToMenu,
    Resume,
    ReturnToMainMenu,
    Exit,
}

/// Pulsante UI con azione semantica associata.
#[derive(Component)]
pub struct UiButton {
    pub action: UiButtonAction,
}

/// Genera un pulsante con label e stile coerente col tema, attaccato a `parent`.
///
/// Ritorna l'entity del pulsante (utile per test o per riferimenti futuri).
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
