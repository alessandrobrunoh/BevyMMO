//! Character selection screen shown after account authentication.

use bevy::prelude::*;

use bevymmo_client::stdb::CharacterRoster;

use crate::game_state::{AuthState, AuthStatus, Screen, MAX_CHARACTERS_PER_ACCOUNT};
use crate::ui::button::{spawn_button, UiButtonAction};
use crate::ui::character_roster::{spawn_roster_list, SelectedRosterEntry};

use crate::ui::text_input::spawn_text_input;
use crate::ui::theme::{
    menu_screen_root_node, ornate_menu_panel_content_node, spawn_menu_screen_background,
    spawn_ornate_menu_panel, UiTheme,
};

#[derive(Component)]
pub struct MainMenuUi;

#[derive(Component)]
pub struct PlayerNameInput;

#[derive(Component)]
struct CreateCharacterUi;

#[derive(Component)]
pub struct MainMenuConnectionFailure;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_main_menu);
        app.add_systems(
            Update,
            (
                update_main_menu_visibility
                    .run_if(state_changed::<Screen>.or_eager(resource_changed::<AuthState>)),
                update_create_character_visibility.run_if(
                    resource_changed::<CharacterRoster>
                        .or_eager(resource_changed::<SelectedRosterEntry>),
                ),
            ),
        );
    }
}

fn setup_main_menu(mut commands: Commands, theme: Res<UiTheme>, asset_server: Res<AssetServer>) {
    // Hidden until authenticated. Do not put this on `menu_screen_root_node`
    // (login shares it and must spawn visible).
    let mut root_node = menu_screen_root_node();
    root_node.display = Display::None;
    let root = commands
        .spawn((root_node, BackgroundColor(theme.screen_bg), MainMenuUi))
        .id();

    spawn_menu_screen_background(&mut commands, root, &asset_server);

    let panel = spawn_ornate_menu_panel(&mut commands, root, &asset_server);

    let content = commands
        .spawn(Node {
            align_items: AlignItems::Stretch,
            row_gap: Val::Px(12.0),
            ..ornate_menu_panel_content_node()
        })
        .id();
    commands.entity(panel).add_child(content);

    spawn_roster_list(&mut commands, content);

    let create_character = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                flex_shrink: 0.0,
                ..default()
            },
            CreateCharacterUi,
        ))
        .id();
    commands.entity(content).add_child(create_character);

    let name_field = spawn_text_input(
        &mut commands,
        create_character,
        "New character name",
        16,
        &theme,
        &asset_server,
    );
    commands.entity(name_field).insert(PlayerNameInput);

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
    commands.entity(content).add_child(failure_text);

    let actions = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(6.0),
            flex_shrink: 0.0,
            ..default()
        })
        .id();
    commands.entity(content).add_child(actions);
    spawn_button(
        &mut commands,
        actions,
        "ENTER WORLD",
        UiButtonAction::Play,
        &theme,
        &asset_server,
    );
    spawn_button(
        &mut commands,
        actions,
        "Settings",
        UiButtonAction::OpenSettings,
        &theme,
        &asset_server,
    );
    spawn_button(
        &mut commands,
        actions,
        "Logout",
        UiButtonAction::LogoutAccount,
        &theme,
        &asset_server,
    );
}

fn update_main_menu_visibility(
    screen: Res<State<Screen>>,
    auth: Res<AuthState>,
    mut query: Query<&mut Node, With<MainMenuUi>>,
) {
    let display = if *screen.get() == Screen::MainMenu && auth.0 == AuthStatus::Authenticated {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut query {
        node.display = display;
    }
}

fn create_name_field_display(roster_len: usize, selected: &SelectedRosterEntry) -> Display {
    if roster_len < MAX_CHARACTERS_PER_ACCOUNT
        && !matches!(selected, SelectedRosterEntry::Existing(_))
    {
        Display::Flex
    } else {
        Display::None
    }
}

fn update_create_character_visibility(
    roster: Res<CharacterRoster>,
    selected: Res<SelectedRosterEntry>,
    mut query: Query<&mut Node, With<CreateCharacterUi>>,
) {
    let display = create_name_field_display(roster.len(), &selected);
    for mut node in &mut query {
        node.display = display;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_field_shows_for_create_or_nothing_when_there_is_room() {
        assert_eq!(
            create_name_field_display(0, &SelectedRosterEntry::None),
            Display::Flex
        );
        assert_eq!(
            create_name_field_display(1, &SelectedRosterEntry::Create),
            Display::Flex
        );
        assert_eq!(
            create_name_field_display(1, &SelectedRosterEntry::Existing("Galvdon".into())),
            Display::None
        );
        assert_eq!(
            create_name_field_display(MAX_CHARACTERS_PER_ACCOUNT, &SelectedRosterEntry::None),
            Display::None
        );
        assert_eq!(
            create_name_field_display(MAX_CHARACTERS_PER_ACCOUNT, &SelectedRosterEntry::Create),
            Display::None
        );
    }
}
