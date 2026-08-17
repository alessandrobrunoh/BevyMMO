//! Client application-local screen state and connection intents.
//!
//! Presentation depends on the client crate and reads these resources without
//! owning the transport that mutates them.

use bevy::prelude::*;

/// Reasons why a chosen player name was rejected.
#[derive(Debug, PartialEq, Eq)]
pub enum PlayerNameError {
    TooShort,
    TooLong,
}

/// Local screen displayed by the client.
///
/// `Paused` is only an overlay: it does not pause simulation or network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    MainMenu,
    Settings,
    Connecting,
    InGame,
    Paused,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameScreen(pub Screen);

/// Whether a text-entry field (chat, login form, character name, ...) is
/// currently capturing keyboard input.
///
/// Lives here, in `client`, rather than in `presentation` where the actual
/// text-input components (`TextInput`, chat's own `ChatInput`) are defined,
/// because `client`-crate systems (`stdb::combat_input::send_combat_inputs`,
/// `stdb::plugin::send_move_commands`) need to read it too, and `client` is a
/// dependency of `presentation`, not the other way around. `presentation`
/// keeps this in sync every frame from whichever text field actually exists;
/// see `crate::ui::systems::sync_typing_focus`.
///
/// Gameplay systems that read raw keybinds must `run_if(not_typing)`
/// (`crate::app_state::not_typing`) — without this, typing a chat message or
/// an email/password at login also fires whatever cast/toggle keybind
/// happens to share a letter with what was typed.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TypingFocus(pub bool);

/// Run condition: true when no text field currently has focus. See
/// [`TypingFocus`].
pub fn not_typing(typing: Res<TypingFocus>) -> bool {
    !typing.0
}

#[derive(Resource, Debug, Default)]
pub struct ConnectionRequest(pub Option<ConnectionIntent>);

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionIntent {
    Connect {
        player_name: String,
    },
    Disconnect,
    /// Returns to character select: calls `leave` (marks the active
    /// character offline, does not delete it) and stays authenticated as
    /// the same account — no disconnect. The pause menu's "Leave Character".
    LeaveCharacter,
    /// Ends the account's session (calls `logout`) and returns to the login
    /// form, so a different account can sign in — no disconnect: the same
    /// SpacetimeDB connection just authenticates fresh on the next
    /// `login`/`register`. The character-select screen's "Logout".
    LogoutAccount,
    /// The process is exiting. Disconnect and let the app close once the
    /// disconnect has actually reached the socket (or a short grace period
    /// runs out), instead of tearing the connection down mid-send.
    Shutdown,
}

#[derive(Resource, Debug, Default)]
pub struct ConnectionFailure(pub Option<String>);

/// Where the connection stands with respect to an [`Account`](crate) login.
///
/// Distinct from [`Screen`]: `Screen` is which panel is on-screen, `AuthStatus`
/// is whether the connection is allowed to do anything past the login form.
/// A cached SpacetimeDB token is not, by itself, evidence of either — see the
/// module docs on `stdb::plugin` for why every connection re-authenticates
/// explicitly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AuthStatus {
    #[default]
    LoggedOut,
    Authenticating,
    Authenticated,
    Rejected,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AuthState(pub AuthStatus);

/// The last `register`/`login` rejection reason, shown under the auth form.
/// Cleared on the next attempt, independent of [`ConnectionFailure`] (which is
/// about `join`, a step later in the flow).
#[derive(Resource, Debug, Default)]
pub struct AuthFailure(pub Option<String>);

#[derive(Resource, Debug, Default)]
pub struct AuthRequest(pub Option<AuthIntent>);

/// Mirrors `bevymmo_module::MAX_CHARACTERS_PER_ACCOUNT`. The server enforces
/// the real cap; this is only so the character-select screen knows when to
/// hide the "create a new character" field.
pub const MAX_CHARACTERS_PER_ACCOUNT: usize = 3;

/// Requests deletion of one of the caller's own characters by id. A separate
/// resource from [`ConnectionRequest`]/[`AuthRequest`]: deleting a character
/// is neither a connection-lifecycle action nor an account-auth action.
#[derive(Resource, Debug, Default)]
pub struct DeleteCharacterRequest(pub Option<spacetimedb_sdk::Uuid>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthIntent {
    Register { email: String, password: String },
    Login { email: String, password: String },
}

/// Reasons a typed email fails the client-side shape check.
///
/// Deliberately loose — a full RFC 5321 parser would still accept addresses no
/// mail server would deliver to. The server re-validates authoritatively
/// (`reducers::account::validate_email`); this only gives fast feedback
/// without a round trip for the obviously-wrong cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailError {
    MissingAt,
    EmptyLocalOrDomain,
    DomainMissingDot,
}

/// Normalizes (trim + lowercase) and validates an email's shape.
pub fn validate_email(email: &str) -> Result<String, EmailError> {
    let normalized = email.trim().to_lowercase();
    let Some((local, domain)) = normalized.split_once('@') else {
        return Err(EmailError::MissingAt);
    };
    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return Err(EmailError::EmptyLocalOrDomain);
    }
    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return Err(EmailError::DomainMissingDot);
    }
    Ok(normalized)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordError {
    TooShort,
}

/// Mirrors the server's floor (`reducers::account::MIN_PASSWORD_LEN`). The
/// server is authoritative; this exists only so a too-short password is
/// rejected before a round trip, not to duplicate the full policy.
const MIN_PASSWORD_LEN: usize = 8;

pub fn validate_password(password: &str) -> Result<(), PasswordError> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(PasswordError::TooShort);
    }
    Ok(())
}

/// Normalizes and validates the player's chosen name.
pub fn validate_player_name(name: &str) -> Result<String, PlayerNameError> {
    let name = name.trim();
    let length = name.chars().count();

    if length < 3 {
        return Err(PlayerNameError::TooShort);
    }
    if length > 16 {
        return Err(PlayerNameError::TooLong);
    }

    Ok(name.to_owned())
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameScreen>();
        app.init_resource::<ConnectionRequest>();
        app.init_resource::<ConnectionFailure>();
        app.init_resource::<AuthState>();
        app.init_resource::<AuthFailure>();
        app.init_resource::<AuthRequest>();
        app.init_resource::<DeleteCharacterRequest>();
        app.init_resource::<TypingFocus>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_name_is_trimmed_and_must_be_between_three_and_sixteen_characters() {
        assert_eq!(validate_player_name("  Ada  "), Ok("Ada".to_owned()));
        assert_eq!(validate_player_name("ab"), Err(PlayerNameError::TooShort));
        assert_eq!(
            validate_player_name("abcdefghijklmnopq"),
            Err(PlayerNameError::TooLong)
        );
    }

    #[test]
    fn screen_defaults_to_main_menu() {
        assert_eq!(GameScreen::default().0, Screen::MainMenu);
    }

    #[test]
    fn auth_state_defaults_to_logged_out() {
        assert_eq!(AuthState::default().0, AuthStatus::LoggedOut);
    }

    #[test]
    fn validate_email_normalizes_case_and_whitespace() {
        assert_eq!(
            validate_email("  Someone@Example.COM  "),
            Ok("someone@example.com".to_string())
        );
    }

    #[test]
    fn validate_email_rejects_missing_at_sign() {
        assert_eq!(
            validate_email("no-at-sign.com"),
            Err(EmailError::MissingAt)
        );
    }

    #[test]
    fn validate_email_rejects_empty_local_or_domain() {
        assert_eq!(
            validate_email("@example.com"),
            Err(EmailError::EmptyLocalOrDomain)
        );
        assert_eq!(
            validate_email("user@"),
            Err(EmailError::EmptyLocalOrDomain)
        );
    }

    #[test]
    fn validate_email_rejects_domain_without_dot() {
        assert_eq!(
            validate_email("user@nodomain"),
            Err(EmailError::DomainMissingDot)
        );
    }

    #[test]
    fn validate_password_enforces_minimum_length() {
        assert_eq!(validate_password("1234567"), Err(PasswordError::TooShort));
        assert_eq!(validate_password("12345678"), Ok(()));
    }
}
