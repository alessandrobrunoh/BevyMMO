//! Character-select screen: title, character roster, "create a new
//! character" field, and Play / Settings / Exit buttons.
//!
//! Shown at [`Screen::MainMenu`] only once the connection is authenticated
//! ([`AuthStatus::Authenticated`]); `crate::ui::login::LoginUi` is the other
//! full-screen panel for that same [`Screen`] and occupies the screen
//! beforehand — the two are mutually exclusive, never both `Display::Flex`
//! at once. Spawning happens once in `Startup`; visibility is governed by
//! [`update_main_menu_visibility`] and [`update_create_character_visibility`],
//! which change only [`Display`].

use bevy::prelude::*;

use bevymmo_client::stdb::CharacterRoster;

use crate::game_state::{AuthState, AuthStatus, GameScreen, MAX_CHARACTERS_PER_ACCOUNT, Screen};
use crate::ui::button::{spawn_button, UiButtonAction};
use crate::ui::character_roster::spawn_roster_list;
use crate::ui::text::spawn_text;
use crate::ui::text_input::spawn_text_input;
use crate::ui::theme::UiTheme;

/// Marker: main menu root.
#[derive(Component)]
pub struct MainMenuUi;

/// Marker: the character-name field. Distinguishes it from the login form's
/// email/password fields now that more than one [`crate::ui::text_input::TextInput`]
/// can exist at once.
#[derive(Component)]
pub struct PlayerNameInput;

/// Marker: the "create a new character" subtree (name field, its failure
/// text, and the create button). Hidden once the account already owns
/// [`MAX_CHARACTERS_PER_ACCOUNT`] characters — the roster above is full, and
/// the server would reject a new one anyway (see `reducers::lifecycle::join`).
#[derive(Component)]
struct CreateCharacterUi;

/// Marker: text displaying any [`crate::game_state::ConnectionFailure`]
/// under the name field. It is separate from the validation error of
/// [`crate::ui::text_input::TextInput`] and does not overwrite it.
#[derive(Component)]
pub struct MainMenuConnectionFailure;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_main_menu);
        app.add_systems(
            Update,
            (update_main_menu_visibility, update_create_character_visibility),
        );
    }
}

fn setup_main_menu(mut commands: Commands, theme: Res<UiTheme>, asset_server: Res<AssetServer>) {
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

    spawn_roster_list(&mut commands, root);

    let create_character = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            CreateCharacterUi,
        ))
        .id();
    commands.entity(root).add_child(create_character);

    let name_field = spawn_text_input(&mut commands, create_character, "Player name", 16, &theme);
    commands.entity(name_field).insert(PlayerNameInput);

    // Slot for connection failure message (separate from the validation error
    // of the name field). Updated by `update_connection_failure`.
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
    commands.entity(create_character).add_child(failure_text);

    spawn_button(
        &mut commands,
        create_character,
        "Create",
        UiButtonAction::Play,
        &theme,
        &asset_server,
    );

    spawn_button(
        &mut commands,
        root,
        "Settings",
        UiButtonAction::OpenSettings,
        &theme,
        &asset_server,
    );
    spawn_button(
        &mut commands,
        root,
        "Exit",
        UiButtonAction::Exit,
        &theme,
        &asset_server,
    );
}

fn update_main_menu_visibility(
    screen: Res<GameScreen>,
    auth: Res<AuthState>,
    mut query: Query<&mut Node, With<MainMenuUi>>,
) {
    let display = if matches!(screen.0, Screen::MainMenu) && auth.0 == AuthStatus::Authenticated {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}

/// Hides the "create a new character" field once the roster is full.
fn update_create_character_visibility(
    roster: Res<CharacterRoster>,
    mut query: Query<&mut Node, With<CreateCharacterUi>>,
) {
    let display = if roster.len() < MAX_CHARACTERS_PER_ACCOUNT {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}
