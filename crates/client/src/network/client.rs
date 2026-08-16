use bevy::prelude::*;
use core::time::Duration;
use std::net::SocketAddr;

use crate::app_state::{
    ConnectionFailure, ConnectionIntent, ConnectionRequest, GameScreen, Screen,
};
use bevymmo_network::network::mode::has_client;
use bevymmo_network::network::protocol::{Channel2, JoinRequest};
use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::netcode::NetcodeClient;
use lightyear::prelude::client::{InputDelayConfig, InputTimelineConfig};
use lightyear::prelude::*;

use crate::network::runtime::{
    handle_controlled_spawn, handle_interpolated_spawn, handle_predicted_spawn,
    lower_controlled_saturation, receive_messages, receive_spell_visual_effects,
    DisconnectRequested, PendingClientCleanup, PendingJoinRequest,
};
use crate::network::types::{ClientConnectionConfig, ConnectedClient};

pub struct ClientTransportPlugins {
    pub client_id: u64,
    pub server_addr: SocketAddr,
    pub client_addr: SocketAddr,
    pub tick_duration: Duration,
}

impl Plugin for ClientTransportPlugins {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClientConnectionConfig {
            client_id: self.client_id,
            server_addr: self.server_addr,
            client_addr: self.client_addr,
        });
        app.init_resource::<PendingJoinRequest>();

        app.add_plugins(lightyear::prelude::client::ClientPlugins {
            tick_duration: self.tick_duration,
        });

        app.add_systems(
            Update,
            (connect_on_intent, disconnect_on_intent)
                .chain()
                .run_if(has_client),
        );

        app.add_systems(Update, cleanup_disconnected_clients.run_if(has_client));
        app.add_systems(Update, receive_messages);
        app.add_systems(Update, receive_spell_visual_effects.run_if(has_client));
        app.add_systems(Update, lower_controlled_saturation);

        app.add_observer(handle_connected);
        app.add_observer(handle_disconnected);
        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
    }
}

fn connect_on_intent(
    mut request: ResMut<ConnectionRequest>,
    mut pending: ResMut<PendingJoinRequest>,
    mut failure: ResMut<ConnectionFailure>,
    mut screen: ResMut<GameScreen>,
    config: Res<ClientConnectionConfig>,
    clients: Query<Entity, With<Client>>,
    mut commands: Commands,
) {
    let Some(ConnectionIntent::Connect { player_name }) = request.0.as_ref() else {
        return;
    };
    let player_name = player_name.clone();
    request.0 = None;
    failure.0 = None;

    if !clients.is_empty() {
        return;
    }

    let auth = Authentication::Manual {
        server_addr: config.server_addr,
        client_id: config.client_id,
        private_key: [0; 32],
        protocol_id: 0,
    };

    let netcode_config = NetcodeConfig {
        client_timeout_secs: 10,
        ..default()
    };

    let netcode = match NetcodeClient::new(auth, netcode_config) {
        Ok(n) => n,
        Err(e) => {
            error!("Failed to create NetcodeClient: {e:?}");
            failure.0 = Some("Unable to start the network client.".to_owned());
            screen.0 = Screen::MainMenu;
            return;
        }
    };

    let client = commands
        .spawn((
            Client::default(),
            Link::new(None),
            LocalAddr(config.client_addr),
            PeerAddr(config.server_addr),
            PredictionManager::default(),
            netcode,
            UdpIo::default(),
            InputTimelineConfig::default().with_input_delay(InputDelayConfig::no_input_delay()),
        ))
        .id();

    info!("Spawning client entity {client:?} and starting connection");
    commands.trigger(Connect { entity: client });

    pending.0 = Some(player_name);
    screen.0 = Screen::Connecting;
}

fn disconnect_on_intent(
    mut request: ResMut<ConnectionRequest>,
    clients: Query<Entity, With<Client>>,
    mut commands: Commands,
) {
    if !matches!(request.0.as_ref(), Some(ConnectionIntent::Disconnect)) {
        return;
    }
    request.0 = None;
    let Some(client) = clients.iter().next() else {
        return;
    };
    info!("Triggering disconnect on client entity {client:?}");
    commands.entity(client).insert(DisconnectRequested);
    commands.trigger(Disconnect { entity: client });
}

fn handle_connected(
    trigger: On<Add, Connected>,
    mut commands: Commands,
    mut sender: Query<&mut MessageSender<JoinRequest>>,
    pending: Res<PendingJoinRequest>,
    mut screen: ResMut<GameScreen>,
) {
    let Ok(mut sender) = sender.get_mut(trigger.entity) else {
        return;
    };
    let Some(player_name) = pending.0.clone() else {
        return;
    };
    info!("Client connected: sending JoinRequest for {player_name:?}");
    commands.entity(trigger.entity).insert(ConnectedClient);
    sender.send::<Channel2>(JoinRequest { player_name });
    screen.0 = Screen::InGame;
}

fn handle_disconnected(
    trigger: On<Add, Disconnected>,
    disconnected: Query<&Disconnected>,
    lifecycle: Query<(), Or<(With<ConnectedClient>, With<DisconnectRequested>)>>,
    mut pending: ResMut<PendingJoinRequest>,
    mut failure: ResMut<ConnectionFailure>,
    mut screen: ResMut<GameScreen>,
    mut commands: Commands,
) {
    let reason = disconnected
        .get(trigger.entity)
        .ok()
        .and_then(|d| d.reason.clone());

    if reason.is_none() && lifecycle.get(trigger.entity).is_err() {
        return;
    }

    info!("Client disconnected (reason: {reason:?})");

    pending.0 = None;
    if let Some(reason) = reason {
        failure.0 = Some(reason);
    }
    screen.0 = Screen::MainMenu;
    commands.entity(trigger.entity).insert(PendingClientCleanup);
}

fn cleanup_disconnected_clients(
    clients: Query<Entity, With<PendingClientCleanup>>,
    mut commands: Commands,
) {
    for client in clients.iter() {
        commands.entity(client).despawn();
    }
}
