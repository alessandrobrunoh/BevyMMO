//! Client-side network and input logic for BevyMMO.
//!
//! Hosts client-only helpers such as key mapping and targeting systems.
//! Transport extraction is still in progress during the crate-split migration.

pub mod app_state;
pub mod input;
pub mod local_player;
pub mod movement;
pub mod network;
pub mod player_movement;
pub mod server_feed;
pub mod stdb;
pub mod targeting;
pub mod user_settings;

pub mod prelude {
    pub use crate::app_state::{
        ConnectionFailure, ConnectionIntent, ConnectionRequest, GameScreen, GameStatePlugin,
        PlayerNameError, Screen,
    };
    pub use crate::user_settings::{GameSettingsResource, KeyAction};
    pub use crate::local_player::LocalPlayer;
    pub use crate::network::client::ClientTransportPlugins;
    pub use crate::network::runtime::{
        handle_controlled_spawn, handle_interpolated_spawn, handle_predicted_spawn,
        lower_controlled_saturation, receive_messages, receive_spell_visual_effects,
        DisconnectRequested, PendingClientCleanup, PendingJoinRequest,
    };
    pub use crate::network::types::{ClientConnectionConfig, ConnectedClient};
    pub use crate::targeting::TargetingPlugin;
}
