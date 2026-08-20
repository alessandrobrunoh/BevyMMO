//! Bottom-left global chat widget.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonInput, ButtonState};
use bevy::prelude::*;
use bevymmo_client::server_feed::{ChatLine, ServerNotice};
use bevymmo_client::stdb::{commands, PartyRoster, StdbConnection};

use crate::game_state::{in_unpaused_gameplay, PauseOverlay, Screen};
use crate::ui::scrollbar::spawn_scroll_view_with_content;
use crate::ui::theme::UiTheme;

const MAX_LOCAL_CHARS: usize = 240;
const MAX_VISIBLE_LINES: usize = 30;

#[derive(Component)]
struct ChatRoot;

#[derive(Component)]
struct ChatHistory;

/// `pub(crate)` (not private) so `ui::systems::sync_typing_focus` can read
/// `focused` alongside `TextInput`'s — see [`bevymmo_client::app_state::TypingFocus`].
#[derive(Component)]
pub(crate) struct ChatInput {
    value: String,
    pub(crate) focused: bool,
}

#[derive(Component)]
struct ChatInputText;

#[derive(Resource, Default)]
struct ChatUi {
    input: Option<Entity>,
    history: Option<Entity>,
}

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatUi>();
        app.add_systems(Startup, setup_chat);
        app.add_systems(
            Update,
            (
                sync_chat_visibility
                    .run_if(state_changed::<Screen>.or_eager(state_changed::<PauseOverlay>)),
                focus_chat_on_enter.run_if(in_unpaused_gameplay),
                focus_chat_on_click,
                defocus_chat_on_world_click,
                edit_chat_input,
                update_chat_input_display,
                collect_chat_lines,
            )
                .chain(),
        );
    }
}

fn setup_chat(mut commands: Commands, mut chat: ResMut<ChatUi>, theme: Res<UiTheme>) {
    let root = commands
        .spawn((
            ChatRoot,
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(560.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                // Default screen is MainMenu; hide until unpaused InGame.
                display: Display::None,
                ..default()
            },
        ))
        .id();

    let (history_wrapper, history) =
        spawn_scroll_view_with_content(&mut commands, root, &theme, |commands| {
            commands
                .spawn((
                    ChatHistory,
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id()
        });
    commands.entity(history_wrapper).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Px(280.0),
        flex_direction: FlexDirection::Row,
        ..default()
    });

    let input = commands
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(38.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(theme.input_bg),
            BorderColor::all(theme.input_border),
            ChatInput {
                value: String::new(),
                focused: false,
            },
        ))
        .id();
    commands.entity(root).add_child(input);

    let input_text = commands
        .spawn((
            Text::new("Press Enter to chat…"),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size),
                ..default()
            },
            TextColor(theme.muted_text_color),
            ChatInputText,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(input).add_child(input_text);

    chat.input = Some(input);
    chat.history = Some(history);
}

fn sync_chat_visibility(
    screen: Res<State<Screen>>,
    pause: Option<Res<State<PauseOverlay>>>,
    mut roots: Query<&mut Node, With<ChatRoot>>,
) {
    let display = if in_unpaused_gameplay(screen, pause) {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in roots.iter_mut() {
        node.display = display;
    }
}

fn focus_chat_on_enter(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<ChatUi>,
    mut inputs: Query<&mut ChatInput>,
) {
    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    let Some(input_entity) = chat.input else {
        return;
    };
    let Ok(mut input) = inputs.get_mut(input_entity) else {
        return;
    };
    if !input.focused {
        input.focused = true;
    }
}

fn focus_chat_on_click(
    interactions: Query<&Interaction, (With<ChatInput>, Changed<Interaction>)>,
    mut inputs: Query<&mut ChatInput>,
) {
    for interaction in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        for mut input in inputs.iter_mut() {
            input.focused = true;
        }
    }
}

/// Releases chat focus on a click that landed on the game world rather than
/// on any UI element — the same click that, e.g., sends a move command.
/// Without this, clicking to move while chat is focused left the chat
/// silently eating the next several keystrokes meant for gameplay.
fn defocus_chat_on_world_click(
    mouse: Res<ButtonInput<MouseButton>>,
    ui_interactions: Query<&Interaction>,
    mut inputs: Query<&mut ChatInput>,
) {
    if !(mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right)) {
        return;
    }
    // A `Pressed` interaction anywhere in the UI means this click landed on
    // some clickable element — possibly chat's own input, already handled by
    // `focus_chat_on_click` — not on the game world.
    if ui_interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }
    for mut input in inputs.iter_mut() {
        if input.focused {
            input.focused = false;
        }
    }
}

fn edit_chat_input(
    mut events: MessageReader<KeyboardInput>,
    screen: Res<State<Screen>>,
    pause: Option<Res<State<PauseOverlay>>>,
    chat: Res<ChatUi>,
    mut inputs: Query<&mut ChatInput>,
    conn: Option<Res<StdbConnection>>,
    mut notices: MessageWriter<ServerNotice>,
    roster: Option<Res<PartyRoster>>,
    mut chat_lines: MessageWriter<ChatLine>,
) {
    if !in_unpaused_gameplay(screen, pause) {
        events.clear();
        return;
    }
    let Some(input_entity) = chat.input else {
        events.clear();
        return;
    };
    let Ok(mut input) = inputs.get_mut(input_entity) else {
        events.clear();
        return;
    };
    if !input.focused {
        events.clear();
        return;
    }

    for event in events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Backspace => {
                input.value.pop();
            }
            Key::Escape => {
                input.focused = false;
            }
            Key::Enter => {
                let message = input.value.trim().to_string();
                if message.is_empty() {
                    input.focused = false;
                    continue;
                }
                // `/party ...` branches off before `send_chat_message` ever
                // sees it. Anything that is not this exact prefix (including
                // `/partyfoo`, which is not a space or end-of-string after
                // `/party`) falls through to the generic-command check below,
                // unchanged from before this branch existed.
                if let Some(party_command) = parse_party_command(&message) {
                    run_party_command(
                        party_command,
                        conn.as_deref(),
                        roster.as_deref(),
                        &mut chat_lines,
                    );
                    input.value.clear();
                    input.focused = false;
                    continue;
                }
                if let Some(command) = message.strip_prefix('/') {
                    // No parser other than `/party` exists yet, so every
                    // other `/`-prefixed command is unrecognized. Refusing it
                    // locally — logged to console and surfaced as a toast —
                    // beats the old behavior of silently broadcasting it to
                    // everyone as literal chat.
                    let command = command.trim();
                    warn!("chat command not recognized: /{command}");
                    notices.write(ServerNotice::error(format!(
                        "Comando sconosciuto: /{command}"
                    )));
                    input.value.clear();
                    input.focused = false;
                    continue;
                }
                if let Some(conn) = conn.as_ref() {
                    if let Err(error) = commands::send_chat_message(conn, message) {
                        warn!("could not send chat message: {error}");
                    } else {
                        input.value.clear();
                    }
                }
                // Sending returns keyboard control to gameplay, same as
                // Escape — a chat message is a single line, not a
                // conversation the player is expected to keep typing into.
                input.focused = false;
            }
            Key::Space if input.value.chars().count() < MAX_LOCAL_CHARS => {
                input.value.push(' ');
            }
            Key::Character(chars) if input.value.chars().count() < MAX_LOCAL_CHARS => {
                for character in chars.chars() {
                    if input.value.chars().count() >= MAX_LOCAL_CHARS {
                        break;
                    }
                    if character.is_ascii_graphic() || character == ' ' {
                        input.value.push(character);
                    }
                }
            }
            _ => {}
        }
    }
}

fn update_chat_input_display(
    theme: Res<UiTheme>,
    chat: Res<ChatUi>,
    inputs: Query<&ChatInput, Changed<ChatInput>>,
    mut text: Query<&mut Text, With<ChatInputText>>,
    mut borders: Query<&mut BorderColor, With<ChatInput>>,
) {
    let Some(input_entity) = chat.input else {
        return;
    };
    let Ok(input) = inputs.get(input_entity) else {
        return;
    };
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    if input.value.is_empty() {
        text.0 = "Press Enter to chat…".to_string();
    } else {
        text.0 = input.value.clone();
    }

    if let Ok(mut border) = borders.single_mut() {
        border.set_all(if input.focused {
            theme.input_border_focused
        } else {
            theme.input_border
        });
    }
}

fn collect_chat_lines(
    mut commands: Commands,
    mut incoming: MessageReader<ChatLine>,
    chat: Res<ChatUi>,
    theme: Res<UiTheme>,
    mut lines: Local<Vec<Entity>>,
) {
    let Some(history) = chat.history else {
        return;
    };

    for line in incoming.read() {
        while lines.len() >= MAX_VISIBLE_LINES {
            commands.entity(lines.remove(0)).despawn();
        }
        let entity = commands
            .spawn((
                Text::new(line.text.clone()),
                TextFont {
                    font_size: FontSize::Px(theme.input_font_size + 4.0),
                    ..default()
                },
                TextColor(theme.text_color),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(history).add_child(entity);
        lines.push(entity);
    }
}

// ---------------------------------------------------------------------------
// `/party ...`
// ---------------------------------------------------------------------------
//
// Added to the existing submit handler above, not a rewrite of it: everything
// that is not this exact prefix keeps going through `send_chat_message`
// exactly as it did before. `/party list` and bare `/party` never reach the
// server — they render from `PartyRoster`, which is built entirely from
// already-subscribed `party`/`party_member` rows.

const PARTY_HELP_TEXT: &str =
    "/party invite <name> | join <name> | accept <name> | decline <name> | leave | list";

#[derive(Debug, Clone, PartialEq, Eq)]
enum PartyCommand {
    Invite(String),
    Join(String),
    Accept(String),
    Decline(String),
    Leave,
    List,
    /// Bare `/party`, or a subcommand this parser does not recognise.
    Help,
}

/// Parses a `/party ...` chat command. `None` means "this is not a `/party`
/// command at all" — including `/partyfoo`, where the character right after
/// `party` is neither whitespace nor the end of the string, so it is not this
/// prefix plus a space, just a message that happens to start with the same
/// six letters. Ordinary chat text falls into the same `None` case.
fn parse_party_command(message: &str) -> Option<PartyCommand> {
    let after_prefix = message.strip_prefix("/party")?;
    if !after_prefix.is_empty() && !after_prefix.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = after_prefix.trim();
    if rest.is_empty() {
        return Some(PartyCommand::Help);
    }
    let mut parts = rest.splitn(2, char::is_whitespace);
    let verb = parts.next().unwrap_or("").to_ascii_lowercase();
    let arg = parts.next().unwrap_or("").trim().to_string();
    Some(match verb.as_str() {
        "invite" => PartyCommand::Invite(arg),
        "join" => PartyCommand::Join(arg),
        "accept" => PartyCommand::Accept(arg),
        "decline" => PartyCommand::Decline(arg),
        "leave" => PartyCommand::Leave,
        "list" => PartyCommand::List,
        _ => PartyCommand::Help,
    })
}

/// `/party list`'s (and bare `/party`'s) rendering, entirely client-side.
fn render_party_list(roster: Option<&PartyRoster>) -> Vec<String> {
    let members = roster.and_then(PartyRoster::my_party);
    let Some(members) = members else {
        return vec!["You are not in a party.".to_string()];
    };
    let mut lines = vec![format!(
        "Party ({} member{}):",
        members.len(),
        if members.len() == 1 { "" } else { "s" }
    )];
    for member in &members {
        let suffix = if member.is_leader { " (leader)" } else { "" };
        lines.push(format!("  {}{suffix}", member.display_name));
    }
    lines
}

/// Sends `call`'s reducer request, if a connection exists, and logs a
/// warning through the same path as every other command wrapper if the
/// request could not even be sent. The module's own rejection (a bad name, a
/// full party, ...) arrives later and renders through the notice log, same
/// as any other reducer call.
fn dispatch_party_command<E: std::fmt::Display>(
    conn: Option<&StdbConnection>,
    action: &'static str,
    call: impl FnOnce(&StdbConnection) -> Result<(), E>,
) {
    let Some(conn) = conn else {
        return;
    };
    if let Err(error) = call(conn) {
        warn!("could not {action}: {error}");
    }
}

fn run_party_command(
    command: PartyCommand,
    conn: Option<&StdbConnection>,
    roster: Option<&PartyRoster>,
    chat_lines: &mut MessageWriter<ChatLine>,
) {
    match command {
        PartyCommand::Help => {
            chat_lines.write(ChatLine {
                text: PARTY_HELP_TEXT.to_string(),
            });
        }
        PartyCommand::List => {
            for line in render_party_list(roster) {
                chat_lines.write(ChatLine { text: line });
            }
        }
        PartyCommand::Invite(name) => {
            dispatch_party_command(conn, "invite", |c| commands::party_invite(c, name));
        }
        PartyCommand::Join(name) => {
            dispatch_party_command(conn, "ask to join", |c| commands::party_join(c, name));
        }
        PartyCommand::Accept(name) => {
            dispatch_party_command(conn, "accept", |c| commands::party_accept(c, name));
        }
        PartyCommand::Decline(name) => {
            dispatch_party_command(conn, "decline", |c| commands::party_decline(c, name));
        }
        PartyCommand::Leave => {
            dispatch_party_command(conn, "leave the party", commands::party_leave);
        }
    }
}

#[cfg(test)]
mod party_command_tests {
    use super::*;

    #[test]
    fn bare_party_is_help() {
        assert_eq!(parse_party_command("/party"), Some(PartyCommand::Help));
        assert_eq!(parse_party_command("/party   "), Some(PartyCommand::Help));
    }

    #[test]
    fn unrecognized_subcommand_is_help() {
        assert_eq!(
            parse_party_command("/party frobnicate"),
            Some(PartyCommand::Help)
        );
    }

    #[test]
    fn list_is_recognized_case_insensitively() {
        assert_eq!(parse_party_command("/party list"), Some(PartyCommand::List));
        assert_eq!(parse_party_command("/party LIST"), Some(PartyCommand::List));
    }

    #[test]
    fn invite_carries_a_trimmed_argument() {
        assert_eq!(
            parse_party_command("/party invite   Bob  "),
            Some(PartyCommand::Invite("Bob".to_string()))
        );
    }

    #[test]
    fn join_accept_decline_carry_their_argument() {
        assert_eq!(
            parse_party_command("/party join Alice"),
            Some(PartyCommand::Join("Alice".to_string()))
        );
        assert_eq!(
            parse_party_command("/party accept Alice"),
            Some(PartyCommand::Accept("Alice".to_string()))
        );
        assert_eq!(
            parse_party_command("/party decline Alice"),
            Some(PartyCommand::Decline("Alice".to_string()))
        );
    }

    #[test]
    fn leave_ignores_a_trailing_argument() {
        assert_eq!(
            parse_party_command("/party leave whatever"),
            Some(PartyCommand::Leave)
        );
    }

    #[test]
    fn ordinary_chat_is_not_a_party_command() {
        assert_eq!(parse_party_command("hello world"), None);
        assert_eq!(parse_party_command("/party's over"), None);
    }

    #[test]
    fn a_message_that_merely_starts_with_the_same_letters_is_not_a_party_command() {
        // Regression coverage: `/partyfoo` must fall through to
        // `send_chat_message` unchanged, per `plans/account-chat-admin.md`
        // Slice 4's existing-behaviour concern.
        assert_eq!(parse_party_command("/partyfoo"), None);
        assert_eq!(parse_party_command("/partying hard tonight"), None);
    }

    #[test]
    fn empty_roster_renders_as_not_in_a_party() {
        assert_eq!(render_party_list(None), vec!["You are not in a party."]);
    }
}
