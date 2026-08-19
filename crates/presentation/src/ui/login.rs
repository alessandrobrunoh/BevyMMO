//! Login/register form.
//!
//! The other full-screen panel for [`Screen::MainMenu`], alongside
//! `crate::ui::main_menu`'s character-select screen — the two are mutually
//! exclusive, gated by opposite conditions on
//! [`AuthStatus::Authenticated`](crate::game_state::AuthStatus), so exactly
//! one is ever visible. Composed of title, email field, password field,
//! Login / Register buttons, and its own Settings / Exit (a player should be
//! able to reach either before logging in). Spawning happens once in
//! `Startup`; visibility is governed by [`update_login_visibility`], which
//! changes only [`Display`].

use bevy::prelude::*;

use crate::game_state::{AuthState, AuthStatus, GameScreen, Screen};
use crate::ui::button::{spawn_button, UiButtonAction};
use crate::ui::text::spawn_text;
use crate::ui::text_input::{spawn_password_input, spawn_text_input};
use crate::ui::theme::{spawn_menu_screen_background, UiTheme};

/// Marker: login form root.
#[derive(Component)]
pub struct LoginUi;

/// Marker: the email field. Distinguishes it from the password field and
/// from the character-name field in `crate::ui::main_menu`.
#[derive(Component)]
pub struct EmailInput;

/// Marker: the password field.
#[derive(Component)]
pub struct PasswordInput;

/// Marker: text displaying the last `register`/`login` rejection
/// ([`crate::game_state::AuthFailure`]).
#[derive(Component)]
pub struct LoginFailureText;

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_login);
        app.add_systems(Update, (update_login_visibility, update_auth_failure_text));
    }
}

fn setup_login(mut commands: Commands, theme: Res<UiTheme>, asset_server: Res<AssetServer>) {
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
            LoginUi,
        ))
        .id();

    spawn_menu_screen_background(&mut commands, root, &asset_server);

    spawn_text(
        &mut commands,
        root,
        "Bevy Lightyear",
        theme.title_font_size,
        theme.text_color,
    );

    let email_field = spawn_text_input(&mut commands, root, "Email", 254, &theme);
    commands.entity(email_field).insert(EmailInput);

    let password_field = spawn_password_input(&mut commands, root, "Password", 256, &theme);
    commands.entity(password_field).insert(PasswordInput);

    let failure_text = commands
        .spawn((
            Text::new(String::new()),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size - 4.0),
                ..default()
            },
            TextColor(theme.error_color),
            LoginFailureText,
        ))
        .id();
    commands.entity(root).add_child(failure_text);

    spawn_button(
        &mut commands,
        root,
        "Login",
        UiButtonAction::Login,
        &theme,
        &asset_server,
    );
    spawn_button(
        &mut commands,
        root,
        "Register",
        UiButtonAction::Register,
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

fn update_login_visibility(
    screen: Res<GameScreen>,
    auth: Res<AuthState>,
    mut query: Query<&mut Node, With<LoginUi>>,
) {
    let display = if matches!(screen.0, Screen::MainMenu) && auth.0 != AuthStatus::Authenticated {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in query.iter_mut() {
        node.display = display;
    }
}

/// Mirrors `crate::ui::systems::update_connection_failure`, for the auth form
/// instead of the character-name form.
fn update_auth_failure_text(
    failure: Res<crate::game_state::AuthFailure>,
    mut query: Query<&mut Text, With<LoginFailureText>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    text.0 = failure.0.clone().unwrap_or_default();
}
