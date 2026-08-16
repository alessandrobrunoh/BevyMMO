//! Shared application-local screen state and connection intents.
//!
//! These types are pure client/application state, but they are not tied to
//! rendering. Keeping them in `bevymmo_shared` lets both the client transport
//! crate and the presentation crate agree on the same resources without a
//! forbidden `client -> presentation` dependency.

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

#[derive(Resource, Debug, Default)]
pub struct ConnectionRequest(pub Option<ConnectionIntent>);

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionIntent {
    Connect {
        player_name: String,
    },
    Disconnect,
    /// Disconnect and discard the locally cached SpacetimeDB identity.
    Logout,
}

#[derive(Resource, Debug, Default)]
pub struct ConnectionFailure(pub Option<String>);

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
}
