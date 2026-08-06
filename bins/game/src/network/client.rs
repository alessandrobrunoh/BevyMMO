//! Client network facade.
//!
//! The source of truth for the transport/lifecycle logic now lives in
//! `bevymmo_client::network::client`. This root module composes that transport
//! plugin with the presentation-side spell input system during the crate-split
//! migration.

use bevy::prelude::*;
use core::time::Duration;
use std::net::SocketAddr;

use bevymmo_client::network::client::ClientTransportPlugins;
use bevymmo_presentation::spells::input::cast_spells_on_key;
use bevymmo_shared::network::mode::has_client;

pub use bevymmo_client::network::runtime::{
    handle_controlled_spawn, handle_interpolated_spawn, handle_predicted_spawn,
    lower_controlled_saturation, receive_messages, receive_spell_visual_effects,
    DisconnectRequested, PendingClientCleanup, PendingJoinRequest,
};
pub use bevymmo_client::network::types::{ClientConnectionConfig, ConnectedClient};

pub struct ClientPlugins {
    pub client_id: u64,
    pub server_addr: SocketAddr,
    pub client_addr: SocketAddr,
    pub tick_duration: Duration,
}

impl Plugin for ClientPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientTransportPlugins {
            client_id: self.client_id,
            server_addr: self.server_addr,
            client_addr: self.client_addr,
            tick_duration: self.tick_duration,
        });
        app.add_systems(Update, cast_spells_on_key.run_if(has_client));
    }
}
