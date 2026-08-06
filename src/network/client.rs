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
use crate::plugins::key_mapping::KeyBindings;
use crate::plugins::spells::{SpellHudCooldownStarted, SpellHudState};
use crate::plugins::targeting::CurrentTarget;

/// Impostazioni di connessione del client, conservate come risorsa.
#[derive(Resource)]
pub struct ClientConnectionConfig {
    /// Deve essere unico tra client connessi allo stesso server Netcode.
    pub client_id: u64,
    pub server_addr: SocketAddr,
    pub client_addr: SocketAddr,
}

/// Nome validato in attesa di essere inviato al server.
///
/// Viene popolato quando l'utente richiede la connessione e consumato
/// non appena il `Client` lightyear passa nello stato `Connected`.
#[derive(Resource, Default, Debug)]
pub struct PendingJoinRequest(pub Option<String>);

/// Il link ha completato almeno una connessione Lightyear. Distingue un vero
/// disconnect dallo stato `Disconnected` iniziale creato da `Link::new(None)`.
#[derive(Component)]
pub(crate) struct ConnectedClient;

/// L'utente ha richiesto esplicitamente il disconnect mentre il link era attivo.
#[derive(Component)]
struct DisconnectRequested;

/// Cleanup differito: evita di despawnare il link nello stesso frame in cui
/// Lightyear applica i propri comandi di teardown.
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

        // Il client non viene più spawnato in `Startup`: la socket UDP esiste
        // solo dopo che l'utente ha richiesto esplicitamente la connessione.
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

// Consuma l'intento `Connect`: crea l'entità `Client`, apre la socket UDP e
// avvia la connessione. Salva il nome validato per inviarlo dopo `Connected`.
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

    // Evita doppie connessioni se l'utente clicca Play più volte.
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

// Consuma l'intento `Disconnect`: triggera la disconnessione lightyear sul
// `Client` corrente. Il cleanup locale vero e proprio avviene nell'observer
// `handle_disconnected`.
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

// Non appena il client è connesso, invia la `JoinRequest` col nome validato
// e passa allo schermo `InGame`.
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

// Su disconnect (manuale o da errore di rete), ripulisce lo stato pendente
// e riporta la UI al MainMenu, eventualmente segnalando il motivo di fallimento.
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

    // `Link::new(None)` riceve inizialmente `Disconnected` senza motivo: non è
    // un fallimento e il client deve restare vivo per poter ricevere `Connect`.
    if reason.is_none() && lifecycle.get(trigger.entity).is_err() {
        return;
    }

    info!("Client disconnected (reason: {reason:?})");

    pending.0 = None;
    if let Some(reason) = reason {
        failure.0 = Some(reason);
    }
    screen.0 = Screen::MainMenu;

    // Lightyear rimuove ancora componenti dal link in questo frame. Il despawn
    // viene effettuato dal sistema dedicato al frame successivo.
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

fn cast_spells_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    bindings: Option<Res<KeyBindings>>,
    screen: Res<GameScreen>,
    hud_state: Res<SpellHudState>,
    current_target: Res<CurrentTarget>,
    target_ids: Query<&NetworkEntityId>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    controlled_players: Query<&crate::plugins::spells::Spellbook, With<Controlled>>,
    mut senders: Query<&mut MessageSender<SpellCastCommand>, With<ConnectedClient>>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
    registry: Res<crate::plugins::spells::SpellRegistry>,
) {
    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        return;
    }
    let (Some(keys), Some(bindings)) = (keys, bindings) else {
        return;
    };

    let Ok(spellbook) = controlled_players.single() else {
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

    for spell_id in spellbook.spells.iter() {
        let Some(&key) = bindings.spells.get(spell_id) else {
            continue;
        };

        if keys.just_pressed(key) {
            if hud_state.is_on_cooldown(spell_id) {
                continue;
            }

            for mut sender in senders.iter_mut() {
                sender.send::<Channel2>(SpellCastCommand {
                    spell_id: spell_id.0.to_string(),
                    target_position,
                    target_id,
                });
            }

            if let Some(spell_def) = registry.get(spell_id) {
                hud_cooldowns.write(SpellHudCooldownStarted {
                    spell_id: spell_id.clone(),
                    cooldown_seconds: spell_def.config().cooldown_seconds,
                });
            }
        }
    }
}

// Riduce la saturazione sulle entità predette, così il player locale è
// visivamente distinto.
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

// Riduce la saturazione del player controllato (locale) ancora di più.
// Garantito girare solo dopo che il marker `Controlled` è stato aggiunto.
fn lower_controlled_saturation(mut controlled: Query<&mut EntityColor, Added<Controlled>>) {
    for mut color in controlled.iter_mut() {
        let hsva = Hsva {
            saturation: 0.2,
            ..Hsva::from(color.0)
        };
        color.0 = bevy::color::Color::from(hsva);
    }
}

// Aggiunge `InputMarker` al player controllato (locale).
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

// Riduce la saturazione sulle entità interpolate (altri player / altre entità).
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

// Riceve i messaggi dal server.
fn receive_messages(mut receiver: Single<&mut MessageReceiver<PlayerMessage>>) {
    for message in receiver.receive() {
        info!("Received message: {:?}", message);
    }
}

// Converte i messaggi visual server -> client in messaggi Bevy locali letti dai
// sistemi di presentazione in `plugins::spells::effects`.
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
