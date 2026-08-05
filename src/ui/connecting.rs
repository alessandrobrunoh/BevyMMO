//! Schermata `Connecting`: overlay minimale con solo il testo "Connecting…".
//!
//! Visibile solo quando [`crate::game_state::Screen`] è `Connecting`. Lo spawn
//! avviene una sola volta in `Startup`; la visibilità è gestita cambiando solo
//! [`Display`] sul nodo root.

use bevy::prelude::*;

use crate::game_state::{GameScreen, Screen};
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

/// Marker: root della schermata `Connecting`.
#[derive(Component)]
pub struct ConnectingUi;

pub struct ConnectingPlugin;

impl Plugin for ConnectingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_connecting);
        app.add_systems(Update, update_connecting_visibility);
    }
}

fn setup_connecting(mut commands: Commands, theme: Res<UiTheme>) {
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(theme.screen_bg),
            ConnectingUi,
        ))
        .id();

    spawn_text(
        &mut commands,
        root,
        "Connecting…",
        theme.title_font_size,
        theme.text_color,
    );
}

fn update_connecting_visibility(
    screen: Res<GameScreen>,
    mut query: Query<&mut Node, With<ConnectingUi>>,
) {
    let display = if matches!(screen.0, Screen::Connecting) {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}
