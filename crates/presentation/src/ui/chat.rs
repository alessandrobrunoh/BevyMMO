//! Bottom-left global chat widget.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonInput, ButtonState};
use bevy::prelude::*;
use bevymmo_client::server_feed::{ChatLine, ServerNotice};
use bevymmo_client::stdb::{commands, StdbConnection};

use crate::game_state::{GameScreen, Screen};
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
                sync_chat_visibility,
                focus_chat_on_enter,
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
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(560.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .id();

    let history = commands
        .spawn((
            ChatHistory,
            Node {
                width: Val::Percent(100.0),
                max_height: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                // Without this, lines beyond `max_height` overflow visibly
                // instead of being clipped — see `scrollbar.rs` for the same
                // pattern on the other scrollable panels.
                overflow: Overflow::clip_y(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(history);

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

fn chat_is_active(screen: &GameScreen) -> bool {
    matches!(screen.0, Screen::InGame)
}

fn sync_chat_visibility(
    screen: Res<GameScreen>,
    mut roots: Query<&mut Node, With<ChatRoot>>,
) {
    let display = if chat_is_active(&screen) {
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
    screen: Res<GameScreen>,
    chat: Res<ChatUi>,
    mut inputs: Query<&mut ChatInput>,
) {
    if !chat_is_active(&screen) || !keys.just_pressed(KeyCode::Enter) {
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
    if ui_interactions.iter().any(|interaction| *interaction == Interaction::Pressed) {
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
    screen: Res<GameScreen>,
    chat: Res<ChatUi>,
    mut inputs: Query<&mut ChatInput>,
    conn: Option<Res<StdbConnection>>,
    mut notices: MessageWriter<ServerNotice>,
) {
    if !chat_is_active(&screen) {
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
                if let Some(command) = message.strip_prefix('/') {
                    // No slash-command parser exists yet (no `/party` and
                    // friends), so every command is unrecognized. Refusing
                    // it locally — logged to console and surfaced as a
                    // toast — beats the old behavior of silently
                    // broadcasting "/party" to everyone as literal chat.
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
