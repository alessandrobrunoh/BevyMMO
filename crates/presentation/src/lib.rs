//! Client presentation layer for BevyMMO.
//!
//! Rendering (mesh/material/transform derivation from replicated state),
//! scenes (camera, lights, ground), and presentation-side screen state.
//! UI migration is still in progress.

pub mod game_state;
pub mod renderer;
pub mod scenes;
pub mod spells;
pub mod ui;

pub mod prelude {
    pub use crate::game_state::{
        validate_player_name, ConnectionFailure, ConnectionIntent, ConnectionRequest, GameScreen,
        GameStatePlugin, PlayerNameError, Screen,
    };
    pub use crate::renderer::RendererPlugin;
    pub use crate::scenes::ScenesPlugin;
}
