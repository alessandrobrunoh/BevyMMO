//! Player-facing chat reducers.

use spacetimedb::{reducer, ReducerContext, Table};

use crate::tables::{player, player_message, PlayerMessageEvent};

const MAX_CHAT_MESSAGE_CHARS: usize = 240;

fn validate_chat_message(text: &str) -> Result<&str, String> {
    let message = text.trim();
    let length = message.chars().count();
    if length == 0 {
        return Err("chat message cannot be empty".to_string());
    }
    if length > MAX_CHAT_MESSAGE_CHARS {
        return Err(format!(
            "chat message cannot exceed {MAX_CHAT_MESSAGE_CHARS} characters"
        ));
    }
    if message.chars().any(|character| character.is_control()) {
        return Err("chat message contains unsupported control characters".to_string());
    }
    Ok(message)
}

/// Sends one message to every connected client.
///
/// The display name is resolved from the authenticated caller on the server;
/// clients cannot impersonate another player by choosing the author prefix.
#[reducer]
pub fn send_chat_message(ctx: &ReducerContext, text: String) -> Result<(), String> {
    let message = validate_chat_message(&text)?;

    let player = ctx
        .db
        .player()
        .identity()
        .find(&ctx.sender())
        .ok_or_else(|| "you must join the world before chatting".to_string())?;

    ctx.db.player_message().insert(PlayerMessageEvent {
        target: None,
        text: format!("{}: {}", player.display_name, message),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_validation_trims_and_rejects_empty_input() {
        assert_eq!(validate_chat_message("  hello  ").unwrap(), "hello");
        assert!(validate_chat_message("   ").is_err());
    }

    #[test]
    fn chat_message_validation_rejects_control_characters_and_overflow() {
        assert!(validate_chat_message("hello\nworld").is_err());
        assert!(validate_chat_message(&"x".repeat(MAX_CHAT_MESSAGE_CHARS + 1)).is_err());
    }
}
