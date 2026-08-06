use bevy::prelude::*;
use core::time::Duration;

use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::netcode::NetcodeClient;
use lightyear::prelude::client::{InputDelayConfig, InputTimelineConfig};
use lightyear::prelude::input::native::*;
use lightyear::prelude::*;
use std::net::SocketAddr;

use crate::game_state::{
    ConnectionFailure, ConnectionIntent, ConnectionRequest, GameScreen, Screen,
};
use crate::network::mode::has_client;
use crate::network::protocol::SpellVisualEffect;
use crate::network::protocol::*;
use crate::plugins::spells::{
    cast_bar::ObservedCasts, CastKind, HotbarSlot, SpellHotbar, SpellHudCooldownStarted,
    SpellHudState, SpellRegistry,
};
use crate::plugins::targeting::CurrentTarget;

/// Client connection settings, stored as a resource.
#[derive(Resource)]
pub struct ClientConnectionConfig {
    /// Must be unique among clients connected to the same Netcode server.
    pub client_id: u64,
    pub server_addr: SocketAddr,
    pub client_addr: SocketAddr,
}

/// Validated player name waiting to be sent to the server.
///
/// Populated when the user requests a connection and consumed
/// as soon as the lightyear `Client` enters the `Connected` state.
#[derive(Resource, Default, Debug)]
pub struct PendingJoinRequest(pub Option<String>);

/// The link has completed at least one Lightyear connection. Distinguishes a true
/// disconnect from the initial `Disconnected` state created by `Link::new(None)`.
#[derive(Component)]
pub(crate) struct ConnectedClient;

/// The user explicitly requested a disconnect while the link was active.
#[derive(Component)]
struct DisconnectRequested;

/// Deferred cleanup: avoids despawning the link in the same frame that
/// Lightyear applies its own teardown commands.
#[derive(Component)]
struct PendingClientCleanup;

pub struct ClientPlugins {
    pub client_id: u64,
    pub server_addr: SocketAddr,
    pub client_addr: SocketAddr,
    pub tick_duration: Duration,
}

impl Plugin for ClientPlugins {
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

        // The client is no longer spawned in `Startup`: the UDP socket exists
        // only after the user has explicitly requested connection.
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
        app.add_systems(Update, cast_spells_on_key.run_if(has_client));

        app.add_observer(handle_connected);
        app.add_observer(handle_disconnected);
        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
    }
}

// Consumes the `Connect` intent: creates the `Client` entity, opens the UDP socket, and
// starts the connection. Saves the validated name to send it after `Connected`.
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

    // Prevents duplicate connections if the user clicks Play multiple times.
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

// Consumes the `Disconnect` intent: triggers lightyear disconnect on the
// current `Client`. The actual local cleanup occurs in the `handle_disconnected` observer.
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

// As soon as the client is connected, sends the `JoinRequest` with the validated name
// and transitions to the `InGame` screen.
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

// On disconnect (manual or from network error), cleans up pending state
// and returns the UI to MainMenu, optionally reporting the failure reason.
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

    // `Link::new(None)` initially receives `Disconnected` without a reason: this is not
    // a failure and the client must remain alive to be able to receive `Connect`.
    if reason.is_none() && lifecycle.get(trigger.entity).is_err() {
        return;
    }

    info!("Client disconnected (reason: {reason:?})");

    pending.0 = None;
    if let Some(reason) = reason {
        failure.0 = Some(reason);
    }
    screen.0 = Screen::MainMenu;

    // Lightyear is still removing components from the link in this frame. Despawning
    // is performed by the dedicated system on the next frame.
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

#[allow(clippy::too_many_arguments)]
fn cast_spells_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    screen: Res<GameScreen>,
    hud_state: Res<SpellHudState>,
    current_target: Res<CurrentTarget>,
    target_ids: Query<&NetworkEntityId>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    controlled_players: Query<(&SpellHotbar, &NetworkEntityId), With<Controlled>>,
    observed_casts: Res<ObservedCasts>,
    mut cast_senders: Query<&mut MessageSender<SpellCastCommand>, With<ConnectedClient>>,
    mut release_senders: Query<&mut MessageSender<SpellCastRelease>, With<ConnectedClient>>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
    registry: Res<SpellRegistry>,
) {
    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        return;
    }
    let Some(keys) = keys else {
        return;
    };

    let Ok((hotbar, local_network_id)) = controlled_players.single() else {
        return;
    };

    // Calculate common inputs for spells
    let mut target_position = None;
    if let Ok(window) = windows.single() {
        if let Some(cursor_position) = window.cursor_position() {
            if let Some((camera, camera_transform)) = cameras.iter().next() {
                if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                    if let Some(target) = ray.plane_intersection_point(
                        Vec3::ZERO,
                        bevy::math::primitives::InfinitePlane3d::new(Vec3::Y),
                    ) {
                        target_position = Some(Vec3::new(target.x, 0.0, target.z));
                    }
                }
            }
        }
    }

    let mut target_id = None;
    if let Some(target_entity) = current_target.entity {
        if let Ok(net_id) = target_ids.get(target_entity) {
            target_id = Some(net_id.0);
        }
    }

    let check_slot = |key: KeyCode, slot: HotbarSlot| {
        if keys.just_pressed(key) {
            hotbar.spell_for_slot(slot).cloned()
        } else {
            None
        }
    };

    for (key, slot) in [
        (KeyCode::KeyQ, HotbarSlot::Q),
        (KeyCode::KeyW, HotbarSlot::W),
        (KeyCode::KeyE, HotbarSlot::E),
    ] {
        let Some(spell_id) = check_slot(key, slot) else {
            continue;
        };

        let Some(spell_def) = registry.get(&spell_id) else {
            continue;
        };

        // Channeling spells use press-to-toggle: the first press starts the
        // channel, a second press sends a Release. We rely on the
        // server-authoritative `ObservedCasts` mirror to know whether the
        // local player is already channeling this exact spell, so we don't
        // require the user to hold the key down.
        if spell_def.cast_kind() == CastKind::Channeling {
            let is_channeling_this_spell = observed_casts
                .0
                .get(&local_network_id.0)
                .is_some_and(|cast| cast.spell_id == spell_id.0);

            if is_channeling_this_spell {
                for mut sender in release_senders.iter_mut() {
                    sender.send::<Channel2>(SpellCastRelease {
                        spell_id: spell_id.0.to_string(),
                    });
                }
                continue;
            }

            if hud_state.is_on_cooldown(&spell_id) {
                continue;
            }

            for mut sender in cast_senders.iter_mut() {
                sender.send::<Channel2>(SpellCastCommand {
                    spell_id: spell_id.0.to_string(),
                    target_position,
                    target_id,
                });
            }
            hud_cooldowns.write(SpellHudCooldownStarted {
                spell_id: spell_id.clone(),
                cooldown_seconds: spell_def.config().cooldown_seconds,
            });
            continue;
        }

        // Instant / CastTime: single press starts the cast.
        if hud_state.is_on_cooldown(&spell_id) {
            continue;
        }
        for mut sender in cast_senders.iter_mut() {
            sender.send::<Channel2>(SpellCastCommand {
                spell_id: spell_id.0.to_string(),
                target_position,
                target_id,
            });
        }
        if matches!(spell_def.cast_kind(), CastKind::Instant) {
            hud_cooldowns.write(SpellHudCooldownStarted {
                spell_id: spell_id.clone(),
                cooldown_seconds: spell_def.config().cooldown_seconds,
            });
        }
    }
}

// Reduces saturation on predicted entities so the local player is
// visually distinct.
fn handle_predicted_spawn(
    trigger: On<Add, (PlayerId, Predicted)>,
    mut predicted: Query<&mut EntityColor, Without<Controlled>>,
) {
    if let Ok(mut color) = predicted.get_mut(trigger.entity) {
        let hsva = Hsva {
            saturation: 0.4,
            ..Hsva::from(color.0)
        };
        color.0 = bevy::color::Color::from(hsva);
    }
}

// Reduces saturation of the controlled (local) player even further.
// Guaranteed to run only after the `Controlled` marker has been added.
fn lower_controlled_saturation(mut controlled: Query<&mut EntityColor, Added<Controlled>>) {
    for mut color in controlled.iter_mut() {
        let hsva = Hsva {
            saturation: 0.2,
            ..Hsva::from(color.0)
        };
        color.0 = bevy::color::Color::from(hsva);
    }
}

// Adds `InputMarker` to the controlled (local) player.
fn handle_controlled_spawn(
    trigger: On<Add, Controlled>,
    mut commands: Commands,
    players: Query<&PlayerId, Without<InputMarker<Inputs>>>,
) {
    let entity = trigger.entity;
    let Ok(player_id) = players.get(entity) else {
        return;
    };
    info!("Adding InputMarker to controlled player {entity:?} {player_id:?}");
    commands.entity(entity).insert((
        InputMarker::<Inputs>::default(),
        ActionState::<Inputs>::default(),
    ));
}

// Reduces saturation on interpolated entities (other players / other entities).
fn handle_interpolated_spawn(
    trigger: On<Add, Interpolated>,
    mut interpolated: Query<&mut EntityColor>,
) {
    if let Ok(mut color) = interpolated.get_mut(trigger.entity) {
        let hsva = Hsva {
            saturation: 0.1,
            ..Hsva::from(color.0)
        };
        color.0 = bevy::color::Color::from(hsva);
    }
}

// Receives messages from the server.
fn receive_messages(mut receiver: Single<&mut MessageReceiver<PlayerMessage>>) {
    for message in receiver.receive() {
        info!("Received message: {:?}", message);
    }
}

// Converts server -> client visual messages into local Bevy messages read by
// presentation systems in `plugins::spells::effects`.
fn receive_spell_visual_effects(
    mut receivers: Query<&mut MessageReceiver<SpellVisualEffect>, With<ConnectedClient>>,
    mut local_effects: MessageWriter<SpellVisualEffect>,
) {
    for mut receiver in receivers.iter_mut() {
        for effect in receiver.receive() {
            local_effects.write(effect.clone());
        }
    }
}
