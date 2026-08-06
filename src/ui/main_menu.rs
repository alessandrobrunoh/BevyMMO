//! Schermata di menu principale.
//!
//! Composta da titolo, campo nome e pulsanti Play / Settings / Exit. Lo spawn
//! avviene una sola volta in `Startup`; la visibilità è governata da
//! [`update_main_menu_visibility`] che cambia solo [`Display`].

use bevy::prelude::*;

use crate::game_state::{GameScreen, Screen};
use crate::ui::button::{spawn_button, UiButtonAction};
use crate::ui::text::spawn_text;
use crate::ui::text_input::spawn_text_input;
use crate::ui::theme::UiTheme;

/// Marker: root del menu principale.
#[derive(Component)]
pub struct MainMenuUi;

/// Marker: testo che mostra l'eventuale [`crate::game_state::ConnectionFailure`]
/// sotto il campo nome. È separato dall'errore di validazione del
/// [`crate::ui::text_input::TextInput`] e non lo sovrascrive.
#[derive(Component)]
pub struct MainMenuConnectionFailure;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_main_menu);
        app.add_systems(Update, update_main_menu_visibility);
    }
}

fn setup_main_menu(mut commands: Commands, theme: Res<UiTheme>) {
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
            MainMenuUi,
        ))
        .id();

    spawn_text(
        &mut commands,
        root,
        "Bevy Lightyear",
        theme.title_font_size,
        theme.text_color,
    );

    spawn_text_input(&mut commands, root, "Player name", 16, &theme);

    // Slot per il messaggio di errore di connessione (separato dall'errore di
    // validazione del campo nome). Aggiornato da `update_connection_failure`.
    let failure_text = commands
        .spawn((
            Text::new(String::new()),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size - 4.0),
                ..default()
            },
            TextColor(theme.error_color),
            MainMenuConnectionFailure,
        ))
        .id();
    commands.entity(root).add_child(failure_text);

    spawn_button(&mut commands, root, "Play", UiButtonAction::Play, &theme);
    spawn_button(
        &mut commands,
        root,
        "Settings",
        UiButtonAction::OpenSettings,
        &theme,
    );
    spawn_button(&mut commands, root, "Exit", UiButtonAction::Exit, &theme);
}

fn update_main_menu_visibility(
    screen: Res<GameScreen>,
    mut query: Query<&mut Node, With<MainMenuUi>>,
) {
    let display = if matches!(screen.0, Screen::MainMenu) {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}
