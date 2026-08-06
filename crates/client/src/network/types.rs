use bevy::prelude::*;
use std::net::SocketAddr;

/// Client connection settings, stored as a resource.
#[derive(Resource)]
pub struct ClientConnectionConfig {
    /// Must be unique among clients connected to the same Netcode server.
    pub client_id: u64,
    pub server_addr: SocketAddr,
    pub client_addr: SocketAddr,
}

/// The link has completed at least one Lightyear connection. Distinguishes a
/// true disconnect from the initial `Disconnected` state created by
/// `Link::new(None)`.
#[derive(Component)]
pub struct ConnectedClient;
