//! `Connecting` screen: minimal overlay with only the text "Connecting…".
//!
//! Visible only when [`crate::game_state::Screen`] is `Connecting`. Spawning
//! happens once in `Startup`; visibility is managed by changing only
//! [`Display`] on the root node.

use bevy::prelude::*;

use crate::game_state::Screen;
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

/// Marker: root of the `Connecting` screen.
#[derive(Component)]
pub struct ConnectingUi;

pub struct ConnectingPlugin;

impl Plugin for ConnectingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_connecting);
        app.add_systems(
            Update,
            update_connecting_visibility.run_if(state_changed::<Screen>),
        );
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
    screen: Res<State<Screen>>,
    mut query: Query<&mut Node, With<ConnectingUi>>,
) {
    let display = if *screen.get() == Screen::Connecting {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}
