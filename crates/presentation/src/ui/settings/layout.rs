//! Settings shell: sidebar (tab buttons) + content area hosting the panels.
//!
//! The shell is spawned once at startup; panel visibility is driven by the
//! active tab stored in [`ActiveSettingsTab`]. The shell lives inside the
//! `SettingsUi` root and inherits its visibility toggling.

use bevy::prelude::*;

use crate::ui::button::{spawn_button, UiButtonAction};
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

use crate::ui::settings::state::{GameSettingsResource, SettingsTab};

/// Resource: which tab is currently shown.
#[derive(Resource, Default, Clone, Copy)]
pub struct ActiveSettingsTab(pub SettingsTab);

/// Marker: a sidebar tab button. `tab` identifies the target panel.
#[derive(Component, Clone, Copy)]
pub struct SettingsTabButton {
    pub tab: SettingsTab,
}

/// Marker: content area that hosts the three panels.
#[derive(Component)]
pub struct SettingsContentArea;

/// Spawns the whole settings shell (sidebar + content + bottom Back button)
/// under the given root entity. The panels are built by their respective
/// `spawn_*` functions.
pub fn spawn_settings_shell(
    commands: &mut Commands,
    root: Entity,
    theme: &UiTheme,
    settings: &GameSettingsResource,
    monitors: &Query<&bevy::window::Monitor>,
) {
    // Outer row: [ sidebar | content ]
    let shell = commands
        .spawn(Node {
            width: Val::Percent(80.0),
            height: Val::Percent(80.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(0.0),
            padding: Val::Px(0.0).into(),
            ..default()
        })
        .id();
    commands.entity(root).add_child(shell);

    // --- Sidebar -----------------------------------------------------------
    let sidebar = commands
        .spawn(Node {
            width: Val::Px(200.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .id();
    commands.entity(shell).add_child(sidebar);

    let _ = spawn_text(
        commands,
        sidebar,
        "Settings",
        theme.title_font_size * 0.6,
        theme.text_color,
    );

    for tab in SettingsTab::ALL {
        let button = commands
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(theme.button_bg),
                SettingsTabButton { tab },
            ))
            .id();
        commands.entity(sidebar).add_child(button);

        let label = commands
            .spawn((
                Text::new(tab.label().to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size),
                    ..default()
                },
                TextColor(theme.button_text_color),
            ))
            .id();
        commands.entity(button).add_child(label);
    }

    // Spacer pushing the Back button to the bottom of the sidebar.
    let spacer = commands
        .spawn(Node {
            flex_grow: 1.0,
            ..default()
        })
        .id();
    commands.entity(sidebar).add_child(spacer);

    let _ = spawn_button(
        commands,
        sidebar,
        "Back",
        UiButtonAction::BackToMenu,
        theme,
    );

    // --- Content area ------------------------------------------------------
    let content = commands
        .spawn(Node {
            flex_grow: 1.0,
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    commands.entity(shell).add_child(content);

    let content_area = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip_y(),
                ..default()
            },
            SettingsContentArea,
        ))
        .id();
    commands.entity(content).add_child(content_area);

    // Build the three panels (visibility is set in `update_panel_visibility`).
    let _ = crate::ui::settings::panels::general::spawn_general_panel(
        commands,
        content_area,
        theme,
    );
    let _ = crate::ui::settings::panels::graphics::spawn_graphics_panel(
        commands,
        content_area,
        theme,
        monitors,
        settings,
    );
    let _ = crate::ui::settings::panels::keybinds::spawn_keybinds_panel(
        commands,
        content_area,
        theme,
        &settings.0.keybinds,
    );
}
