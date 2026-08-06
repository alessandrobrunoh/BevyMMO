//! Pause overlay: Resume and Return to Main Menu.
//!
//! Pure UI overlay: does not mutate `Time`, `FixedUpdate`, or network. The
//! `InGame <-> Paused` transition is managed by [`crate::ui::systems::toggle_pause`]
//! (the key configured in `KeyBindings`) and by the buttons themselves.

use bevy::prelude::*;

use crate::game_state::{GameScreen, Screen};
use crate::ui::button::{spawn_button, UiButtonAction};
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

/// Marker: pause overlay root.
#[derive(Component)]
pub struct PauseMenuUi;

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_pause_menu);
        app.add_systems(Update, update_pause_menu_visibility);
    }
}

fn setup_pause_menu(mut commands: Commands, theme: Res<UiTheme>) {
    let backdrop = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            PauseMenuUi,
        ))
        .id();

    let panel = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            padding: UiRect::all(Val::Px(32.0)),
            ..default()
        })
        .id();
    commands.entity(backdrop).add_child(panel);

    spawn_text(
        &mut commands,
        panel,
        "Paused",
        theme.title_font_size,
        theme.text_color,
    );

    spawn_button(
        &mut commands,
        panel,
        "Resume",
        UiButtonAction::Resume,
        &theme,
    );
    spawn_button(
        &mut commands,
        panel,
        "Return to Main Menu",
        UiButtonAction::ReturnToMainMenu,
        &theme,
    );
}

fn update_pause_menu_visibility(
    screen: Res<GameScreen>,
    mut query: Query<&mut Node, With<PauseMenuUi>>,
) {
    let display = if matches!(screen.0, Screen::Paused) {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}
