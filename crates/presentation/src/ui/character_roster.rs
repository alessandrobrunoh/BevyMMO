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

const ROW_BACKGROUND: &str = "ui/extracted_065811/panel_row_01.png";
const PLAY_BUTTON_PATH: &str = "ui/extracted_065811/bar_blue_left_01.png";
const DELETE_BUTTON_PATH: &str = "ui/extracted_065811/bar_neutral_right_01.png";
/// Tall enough that 9-slicing `panel_row_01` does not crush the ring or banner.
const ROW_HEIGHT: f32 = 120.0;
/// Left slice keeps the portrait ring; right slice keeps the baked banner.
const ROW_SLICE: [f32; 4] = [118.0, 102.0, 28.0, 28.0];
/// Clears the 9-slice so name + Play/Delete sit only in the dark center.
const ROW_PAD_LEFT: f32 = 118.0;
const ROW_PAD_RIGHT: f32 = 102.0;
const ROW_PAD_Y: f32 = 24.0;
const ACTION_BUTTON_WIDTH: f32 = 92.0;
const ACTION_BUTTON_HEIGHT: f32 = 32.0;
const ACTION_BUTTON_GAP: f32 = 6.0;

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
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: Val::Px(10.0),
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
                tint_roster_buttons,
            ),
        );
    }
}

fn rebuild_roster_list(
    mut commands: Commands,
    theme: Res<UiTheme>,
    asset_server: Res<AssetServer>,
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
            list.spawn((Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                padding: UiRect::vertical(Val::Px(8.0)),
                ..default()
            },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("No characters yet — create one below."),
                        TextFont {
                            font_size: FontSize::Px(theme.input_font_size - 2.0),
                            ..default()
                        },
                        TextColor(theme.muted_text_color),
                    ));
                });
        });
        return;
    }

    for character in characters {
        spawn_roster_row(&mut commands, list, character, &theme, &asset_server);
    }
}

fn spawn_roster_row(
    commands: &mut Commands,
    parent: Entity,
    character: &RosterCharacter,
    theme: &UiTheme,
    asset_server: &AssetServer,
) {
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(ROW_HEIGHT),
                min_height: Val::Px(ROW_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                // Clear the portrait ring on the left and the baked banner
                // on the right so name + actions sit in the dark center.
                padding: UiRect {
                    left: Val::Px(ROW_PAD_LEFT),
                    right: Val::Px(ROW_PAD_RIGHT),
                    top: Val::Px(ROW_PAD_Y),
                    bottom: Val::Px(ROW_PAD_Y),
                },
                column_gap: Val::Px(12.0),
                overflow: Overflow::clip(),
                ..default()
            },
            ImageNode::new(asset_server.load(ROW_BACKGROUND)).with_mode(NodeImageMode::Sliced(
                TextureSlicer {
                    border: BorderRect::from(ROW_SLICE),
                    center_scale_mode: SliceScaleMode::Stretch,
                    sides_scale_mode: SliceScaleMode::Stretch,
                    max_corner_scale: 1.0,
                },
            )),
        ))
        .id();
    commands.entity(parent).add_child(row);

    let details = commands
        .spawn(Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_width: Val::Px(0.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(4.0),
            overflow: Overflow::clip(),
            ..default()
        })
        .id();
    commands.entity(row).add_child(details);

    commands.entity(details).with_children(|details| {
        details.spawn((
            Text::new(character.display_name.clone()),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size),
                ..default()
            },
            TextColor(theme.text_color),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
        ));
        if character.online {
            details.spawn((
                Text::new("Online".to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.input_font_size - 6.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.85, 0.55)),
            ));
        }
    });

    let actions = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(ACTION_BUTTON_GAP),
            flex_shrink: 0.0,
            ..default()
        })
        .id();
    commands.entity(row).add_child(actions);

    spawn_row_button(
        commands,
        actions,
        "Play",
        RosterPlayButton(character.display_name.clone()),
        theme,
        asset_server,
        PLAY_BUTTON_PATH,
    );
    spawn_row_button(
        commands,
        actions,
        "Delete",
        RosterDeleteButton::Idle(character.character_id),
        theme,
        asset_server,
        DELETE_BUTTON_PATH,
    );
}

/// Compact ornate bar, distinct from `crate::ui::button::spawn_button`
/// (full-size bar driven by the global [`crate::ui::button::UiButtonAction`]
/// dispatch): each roster row needs its own per-instance data instead.
fn spawn_row_button<C: Component + Clone>(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    action: C,
    theme: &UiTheme,
    asset_server: &AssetServer,
    texture: &'static str,
) {
    let button = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(ACTION_BUTTON_WIDTH),
                height: Val::Px(ACTION_BUTTON_HEIGHT),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode::new(asset_server.load(texture)).with_mode(NodeImageMode::Sliced(
                TextureSlicer {
                    border: BorderRect::axes(24.0, 10.0),
                    center_scale_mode: SliceScaleMode::Stretch,
                    sides_scale_mode: SliceScaleMode::Stretch,
                    max_corner_scale: 1.0,
                },
            )),
            action,
        ))
        .id();
    commands.entity(parent).add_child(button);
    commands.entity(button).with_children(|button| {
        button.spawn((
            Text::new(label.to_string()),
            TextFont {
                font_size: FontSize::Px(theme.button_font_size * 0.55),
                ..default()
            },
            TextColor(theme.button_text_color),
        ));
    });
}

fn tint_roster_buttons(
    mut buttons: Query<
        (&Interaction, &mut ImageNode),
        Or<(With<RosterPlayButton>, With<RosterDeleteButton>)>,
    >,
) {
    for (interaction, mut image) in &mut buttons {
        image.color = match *interaction {
            Interaction::Pressed => Color::srgb(0.78, 0.78, 0.78),
            Interaction::Hovered => Color::srgb(1.0, 0.96, 0.82),
            Interaction::None => Color::WHITE,
        };
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_padding_clears_the_nine_slice() {
        assert_eq!(ROW_PAD_LEFT, ROW_SLICE[0]);
        assert_eq!(ROW_PAD_RIGHT, ROW_SLICE[1]);
        assert!(ROW_HEIGHT >= 112.0 && ROW_HEIGHT <= 120.0);
        let inner_height = ROW_HEIGHT - ROW_PAD_Y * 2.0;
        assert!(inner_height >= ACTION_BUTTON_HEIGHT * 2.0 + ACTION_BUTTON_GAP);
    }
}
