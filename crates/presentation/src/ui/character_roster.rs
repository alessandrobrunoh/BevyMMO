//! The account's own characters, with Play/Delete actions.
//!
//! Embedded inside `crate::ui::main_menu`'s character-select screen, above
//! the "create a new character" field — [`crate::ui::main_menu::setup_main_menu`]
//! calls [`spawn_roster_list`] to place it. A separate module because the
//! list rebuilds reactively off [`CharacterRoster`], which changes on its own
//! schedule (row insert/remove) independent of the rest of the screen.

use bevy::prelude::*;

use bevymmo_client::stdb::{CharacterRoster, RosterCharacter};

use crate::game_state::{
    ConnectionIntent, ConnectionRequest, DeleteCharacterRequest, GameScreen, Screen,
};
use crate::ui::theme::UiTheme;

/// Marker: the column that holds one row per character. Rebuilt whole
/// whenever [`CharacterRoster`] changes — the roster is at most
/// [`crate::game_state::MAX_CHARACTERS_PER_ACCOUNT`] entries, so a full
/// rebuild is cheaper than diffing.
#[derive(Component)]
pub struct RosterList;

/// "Play" button for one roster row. Carries the character's `display_name`
/// (not its id) because selecting an existing character goes through the
/// same `join(display_name)` path as creating a new one — see
/// `reducers::lifecycle::join`'s "own name, reactivate" branch.
#[derive(Component, Clone)]
struct RosterPlayButton(String);

/// "Delete" button for one roster row, in one of two states. Starts `Idle`;
/// a first click flips it to `Confirming` and changes its label instead of
/// deleting immediately — an accidental click must not destroy a character.
/// A second click while `Confirming` sends the request. Rebuilding the list
/// (e.g. because a *different* row changed) naturally resets any row still
/// `Idle`, since every row is despawned and respawned from scratch.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum RosterDeleteButton {
    Idle(bevymmo_client::stdb::Uuid),
    Confirming(bevymmo_client::stdb::Uuid),
}

/// Spawns the (initially empty) roster column, attached to `parent`.
/// [`rebuild_roster_list`] fills it in on the next frame that
/// [`CharacterRoster`] is populated.
pub fn spawn_roster_list(commands: &mut Commands, parent: Entity) -> Entity {
    let list = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            RosterList,
        ))
        .id();
    commands.entity(parent).add_child(list);
    list
}

pub struct CharacterRosterPlugin;

impl Plugin for CharacterRosterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                rebuild_roster_list,
                handle_roster_play,
                handle_roster_delete,
            ),
        );
    }
}

fn rebuild_roster_list(
    mut commands: Commands,
    theme: Res<UiTheme>,
    roster: Res<CharacterRoster>,
    list_query: Query<Entity, With<RosterList>>,
) {
    if !roster.is_changed() {
        return;
    }
    let Ok(list) = list_query.single() else {
        return;
    };

    commands.entity(list).despawn_related::<Children>();

    let mut characters: Vec<&RosterCharacter> = roster.iter().collect();
    characters.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    if characters.is_empty() {
        // Otherwise an empty roster is indistinguishable from one that
        // hasn't loaded yet — nothing tells the player their account really
        // has zero characters and the field below is how to fix that.
        commands.entity(list).with_children(|list| {
            list.spawn((
                Text::new("No characters yet — create one below."),
                TextFont {
                    font_size: FontSize::Px(theme.input_font_size - 2.0),
                    ..default()
                },
                TextColor(theme.muted_text_color),
            ));
        });
        return;
    }

    for character in characters {
        spawn_roster_row(&mut commands, list, character, &theme);
    }
}

fn spawn_roster_row(
    commands: &mut Commands,
    parent: Entity,
    character: &RosterCharacter,
    theme: &UiTheme,
) {
    let row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);

    let status = if character.online { " (online)" } else { "" };
    commands.entity(row).with_children(|row| {
        row.spawn((
            Text::new(format!("{}{status}", character.display_name)),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size),
                ..default()
            },
            TextColor(theme.text_color),
        ));
    });

    spawn_row_button(
        commands,
        row,
        "Play",
        RosterPlayButton(character.display_name.clone()),
        theme,
    );
    spawn_row_button(
        commands,
        row,
        "Delete",
        RosterDeleteButton::Idle(character.character_id),
        theme,
    );
}

/// A small flat-color button, distinct from `crate::ui::button::spawn_button`
/// (which is textured and driven by the single global [`crate::ui::button::UiButtonAction`]
/// dispatch): each roster row needs its own per-instance data instead.
fn spawn_row_button<C: Component + Clone>(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    action: C,
    theme: &UiTheme,
) {
    let button = commands
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(theme.button_bg),
            action,
        ))
        .id();
    commands.entity(parent).add_child(button);
    commands.entity(button).with_children(|button| {
        button.spawn((
            Text::new(label.to_string()),
            TextFont {
                font_size: FontSize::Px(theme.button_font_size * 0.6),
                ..default()
            },
            TextColor(theme.button_text_color),
        ));
    });
}

fn handle_roster_play(
    mut screen: ResMut<GameScreen>,
    mut connection_request: ResMut<ConnectionRequest>,
    buttons: Query<(&Interaction, &RosterPlayButton), Changed<Interaction>>,
) {
    for (interaction, button) in buttons.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        screen.0 = Screen::Connecting;
        connection_request.0 = Some(ConnectionIntent::Connect {
            player_name: button.0.clone(),
        });
    }
}

fn handle_roster_delete(
    mut delete_request: ResMut<DeleteCharacterRequest>,
    mut buttons: Query<(&Interaction, &mut RosterDeleteButton, &Children), Changed<Interaction>>,
    mut labels: Query<&mut Text>,
) {
    for (interaction, mut state, children) in buttons.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let next = match *state {
            RosterDeleteButton::Idle(id) => RosterDeleteButton::Confirming(id),
            RosterDeleteButton::Confirming(id) => {
                delete_request.0 = Some(id);
                RosterDeleteButton::Idle(id)
            }
        };
        let label = match next {
            RosterDeleteButton::Idle(_) => "Delete",
            RosterDeleteButton::Confirming(_) => "Confirm?",
        };
        for child in children.iter() {
            if let Ok(mut text) = labels.get_mut(child) {
                text.0 = label.to_string();
            }
        }
        *state = next;
    }
}
