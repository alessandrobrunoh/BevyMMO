//! Client-side network and input logic for BevyMMO.
//!
//! Hosts client-only helpers such as key mapping and targeting systems.
//! Transport extraction is still in progress during the crate-split migration.

pub mod input;
pub mod network;
pub mod targeting;

pub mod prelude {
    pub use crate::input::{KeyBindings, KeyMappingPlugin};
    pub use crate::network::client::ClientTransportPlugins;
    pub use crate::network::runtime::{
        handle_controlled_spawn, handle_interpolated_spawn, handle_predicted_spawn,
        lower_controlled_saturation, receive_messages, receive_spell_visual_effects,
        DisconnectRequested, PendingClientCleanup, PendingJoinRequest,
    };
    pub use crate::network::types::{ClientConnectionConfig, ConnectedClient};
    pub use crate::targeting::TargetingPlugin;
}
