//! Multi-tab settings screen (General / Graphics / Keybinds).
//!
//! Architecture:
//!
//! - [`state`] — plain-data source of truth (`GameSettings` + serialization).
//! - [`layout`] — shell spawn (sidebar + content area).
//! - [`panels`] — one module per tab (`general`, `graphics`, `keybinds`).
//! - [`widgets`] — reusable widgets (`dropdown`, `toggle`, `key_capture`).
//! - [`systems`] — interaction handling, apply to `GameSettingsResource`,
//!   apply to `Window`, persistence.
//!
//! Adding a new tab = new module in `panels/`, new variant in `SettingsTab`,
//! new line in [`SettingsPlugin::build`].

pub mod layout;
pub mod panels;
pub mod state;
pub mod systems;
pub mod widgets;

use bevy::prelude::*;

use crate::game_state::Screen;
use crate::ui::theme::UiTheme;

use layout::ActiveSettingsTab;
use state::{load_settings, GameSettingsResource};

/// Marker: root of the whole settings screen (carries visibility).
#[derive(Component)]
pub struct SettingsUi;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        // Load once at startup; subsequent mutations go through the resource.
        let settings = load_settings();
        app.insert_resource(GameSettingsResource(settings))
            .init_resource::<ActiveSettingsTab>()
            .add_message::<widgets::dropdown::DropdownChanged>()
            .add_message::<widgets::key_capture::KeyBindingChanged>()
            .add_systems(Startup, setup_settings)
            .add_systems(
                Update,
                (
                    update_settings_visibility.run_if(state_changed::<Screen>),
                    systems::update_panel_visibility.run_if(resource_changed::<ActiveSettingsTab>),
                    systems::update_tab_button_visuals,
                    systems::switch_tab_on_click,
                    systems::cycle_dropdown,
                    systems::toggle_on_click,
                    systems::toggle_key_capture_on_click,
                    systems::update_key_capture_input,
                ),
            )
            .add_systems(
                Update,
                (
                    systems::reset_keybinds_on_button,
                    systems::apply_widget_events,
                    systems::apply_graphics_to_window,
                    systems::apply_interface_scale,
                    systems::persist_settings_when_changed,
                ),
            );
    }
}

fn setup_settings(
    mut commands: Commands,
    theme: Res<UiTheme>,
    settings: Res<GameSettingsResource>,
    monitors: Query<&bevy::window::Monitor>,
    asset_server: Res<AssetServer>,
) {
    let root = commands
        .spawn((
            Name::new("Settings Screen"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None, // toggled by update_settings_visibility
                ..default()
            },
            BackgroundColor(theme.screen_bg),
            SettingsUi,
        ))
        .id();

    layout::spawn_settings_shell(
        &mut commands,
        root,
        &theme,
        &settings,
        &monitors,
        &asset_server,
    );
}

/// Toggles the whole settings screen on/off based on `Screen::Settings`.
pub fn update_settings_visibility(
    screen: Res<State<Screen>>,
    mut query: Query<&mut Node, With<SettingsUi>>,
) {
    let display = if *screen.get() == Screen::Settings {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}
