//! Schermata impostazioni (placeholder): titolo + Back.

use bevy::prelude::*;

use crate::game_state::{GameScreen, Screen};
use crate::ui::button::{spawn_button, UiButtonAction};
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

/// Marker: root della schermata impostazioni.
#[derive(Component)]
pub struct SettingsUi;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_settings);
        app.add_systems(Update, update_settings_visibility);
    }
}

fn setup_settings(mut commands: Commands, theme: Res<UiTheme>) {
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(theme.screen_bg),
            SettingsUi,
        ))
        .id();

    spawn_text(
        &mut commands,
        root,
        "Settings",
        theme.title_font_size,
        theme.text_color,
    );

    spawn_button(
        &mut commands,
        root,
        "Back",
        UiButtonAction::BackToMenu,
        &theme,
    );
}

fn update_settings_visibility(
    screen: Res<GameScreen>,
    mut query: Query<&mut Node, With<SettingsUi>>,
) {
    let display = if matches!(screen.0, Screen::Settings) {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}
