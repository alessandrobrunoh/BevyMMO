//! General settings panel.
//!
//! Currently exposes:
//! - Language dropdown (single option: "English"; placeholder for i18n).
//! - Show FPS toggle (wired but only honored when a future FPS overlay
//!   consumes the value).

use bevy::prelude::*;

use crate::ui::theme::UiTheme;

use super::SettingsPanel;
use crate::ui::settings::state::GameSettingsResource;
use crate::ui::settings::widgets::{
    dropdown::{spawn_dropdown, DropdownItem},
    toggle::spawn_toggle,
};
#[derive(Component)]
pub struct GeneralRoot;

pub fn spawn_general_panel(commands: &mut Commands, parent: Entity, theme: &UiTheme) -> Entity {
    let panel = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            GeneralRoot,
            SettingsPanel::General,
        ))
        .id();
    commands.entity(parent).add_child(panel);

    let _ = spawn_dropdown(
        commands,
        panel,
        "language",
        "Language",
        vec![DropdownItem {
            label: "English".to_string(),
            value: "en".to_string(),
        }],
        "en",
        theme,
    );

    let _ = spawn_toggle(
        commands,
        panel,
        "show_fps",
        "Show FPS overlay",
        false,
        theme,
    );

    panel
}

/// Reflects `GameSettingsResource` onto the panel widgets when values change
/// outside the UI (e.g. reset to defaults).
pub fn refresh_general_panel(
    _settings: Res<GameSettingsResource>,
    _root: Query<&GeneralRoot>,
) {
    // Widgets are stateless and read their own component state; no work needed
    // today. Hook left in place so future settings (with non-trivial sync)
    // have a clear extension point.
}
