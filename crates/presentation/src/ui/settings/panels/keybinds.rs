//! Keybinds panel: one `KeyCapture` row per [`KeyAction`], plus a "Reset to
//! defaults" button.

use bevy::prelude::*;

use crate::ui::button::{spawn_button, UiButtonAction};
use crate::ui::theme::UiTheme;

use super::SettingsPanel;
use crate::ui::settings::state::{KeyAction, KeybindSettings};
use crate::ui::settings::widgets::key_capture::spawn_key_capture;

#[derive(Component)]
pub struct KeybindsRoot;

pub fn spawn_keybinds_panel(
    commands: &mut Commands,
    parent: Entity,
    theme: &UiTheme,
    keybinds: &KeybindSettings,
) -> Entity {
    let panel = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            KeybindsRoot,
            SettingsPanel::Keybinds,
        ))
        .id();
    commands.entity(parent).add_child(panel);

    for action in KeyAction::ALL {
        let binding = keybinds.get(action);
        let _ = spawn_key_capture(commands, panel, action, binding, theme);
    }

    let _ = spawn_button(
        commands,
        panel,
        "Reset to defaults",
        UiButtonAction::ResetKeybinds,
        theme,
    );

    panel
}

/// Reflects `KeybindSettings` onto the panel widgets when values change
/// outside the UI (e.g. reset to defaults).
pub fn refresh_keybinds_panel(
    _settings: Res<crate::ui::settings::state::GameSettingsResource>,
    _root: Query<&KeybindsRoot>,
) {
}
