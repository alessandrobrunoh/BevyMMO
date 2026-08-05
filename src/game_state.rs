//! Stato locale della schermata e intenti di connessione del client.

use bevy::prelude::*;

/// Schermata locale mostrata dal client.
///
/// `Paused` è solo un overlay: non ferma la simulazione né la rete.
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
    Connect { player_name: String },
    Disconnect,
}

#[derive(Resource, Debug, Default)]
pub struct ConnectionFailure(pub Option<String>);

#[derive(Debug, PartialEq, Eq)]
pub enum PlayerNameError {
    TooShort,
    TooLong,
}

/// Normalizza e valida il nome scelto dal giocatore.
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
}
