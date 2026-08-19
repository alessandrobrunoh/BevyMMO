//! Login and registration pages sharing one validated authentication form.

use bevy::prelude::*;

use crate::game_state::{AuthState, AuthStatus, GameScreen, Screen};
use crate::ui::button::{spawn_button, UiButtonAction};

use crate::ui::text_input::{spawn_password_input, spawn_text_input};
use crate::ui::theme::{
    menu_screen_root_node, ornate_menu_panel_content_node, spawn_menu_screen_background,
    spawn_ornate_menu_panel, UiTheme,
};

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum AuthPage {
    Login,
    Register,
}

impl Default for AuthPage {
    fn default() -> Self {
        Self::Login
    }
}

#[derive(Component)]
pub struct LoginUi;

#[derive(Component)]
pub struct RegisterUi;

#[derive(Component)]
pub struct EmailInput;

#[derive(Component)]
pub struct PasswordInput;

#[derive(Component)]
pub struct LoginFailureText;

#[derive(Component)]
struct LoginOnlyAction;

#[derive(Component)]
struct RegisterOnlyAction;

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AuthPage>();
        app.add_systems(Startup, setup_login);
        app.add_systems(
            Update,
            (
                update_login_visibility,
                update_auth_page_visibility,
                update_auth_failure_text,
            ),
        );
    }
}

fn setup_login(mut commands: Commands, theme: Res<UiTheme>, asset_server: Res<AssetServer>) {
    let root = commands
        .spawn((
            menu_screen_root_node(),
            BackgroundColor(theme.screen_bg),
            LoginUi,
        ))
        .id();
    spawn_menu_screen_background(&mut commands, root, &asset_server);

    let panel = spawn_ornate_menu_panel(&mut commands, root, &asset_server);

    let content = commands
        .spawn(Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(14.0),
            ..ornate_menu_panel_content_node()
        })
        .id();
    commands.entity(panel).add_child(content);

    let email_field = spawn_text_input(&mut commands, content, "Email", 254, &theme);
    commands.entity(email_field).insert(EmailInput);
    let password_field = spawn_password_input(&mut commands, content, "Password", 256, &theme);
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
    commands.entity(content).add_child(failure_text);

    let login_submit = spawn_button(
        &mut commands,
        content,
        "LOGIN",
        UiButtonAction::Login,
        &theme,
        &asset_server,
    );
    commands.entity(login_submit).insert(LoginOnlyAction);

    let register_submit = spawn_button(
        &mut commands,
        content,
        "CREATE ACCOUNT",
        UiButtonAction::Register,
        &theme,
        &asset_server,
    );
    commands.entity(register_submit).insert(RegisterOnlyAction);

    let login_action = spawn_button(
        &mut commands,
        content,
        "CREATE",
        UiButtonAction::OpenRegister,
        &theme,
        &asset_server,
    );
    commands.entity(login_action).insert(LoginOnlyAction);

    let register_action = spawn_button(
        &mut commands,
        content,
        "BACK",
        UiButtonAction::OpenLogin,
        &theme,
        &asset_server,
    );
    commands.entity(register_action).insert(RegisterOnlyAction);

    spawn_button(
        &mut commands,
        content,
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
    for mut node in &mut query {
        node.display = display;
    }
}

fn update_auth_page_visibility(
    page: Res<AuthPage>,
    mut nodes: Query<
        (
            &mut Node,
            Option<&LoginOnlyAction>,
            Option<&RegisterOnlyAction>,
        ),
        Or<(With<LoginOnlyAction>, With<RegisterOnlyAction>)>,
    >,
) {
    let login = *page == AuthPage::Login;
    for (mut node, login_action, register_action) in &mut nodes {
        if login_action.is_some() {
            node.display = if login { Display::Flex } else { Display::None };
        } else if register_action.is_some() {
            node.display = if login { Display::None } else { Display::Flex };
        }
    }
}

fn update_auth_failure_text(
    failure: Res<crate::game_state::AuthFailure>,
    mut query: Query<&mut Text, With<LoginFailureText>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    text.0 = failure.0.clone().unwrap_or_default();
}
