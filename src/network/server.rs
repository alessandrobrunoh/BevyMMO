use bevy::color::Color;
use bevy::prelude::*;
use core::time::Duration;
use lightyear::connection::client::Connected;
use lightyear::netcode::server_plugin::NetcodeConfig;
use lightyear::prelude::input::native::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use std::net::SocketAddr;
use std::sync::{mpsc, Mutex};

use uuid::Uuid;

use crate::game_state::validate_player_name;
use crate::network::protocol::*;
use crate::plugins::entity::components::{EntityState, PlayerName, SpawnPoint};
use crate::plugins::entity::definition::EntityDefinition;
use crate::plugins::entity::dummy::components::Dummy;
use crate::plugins::entity::enemy::components::Enemy;
use crate::plugins::entity::events::RespawnedEvent;
use crate::plugins::entity::player::components::Player;
use crate::plugins::entity::player::spawn::PLAYER_SPAWN_POINT;
use crate::plugins::entity::spawn::{spawn_entity, GameEntityBundle};
use crate::plugins::persistence::{
    normalize_name, PersistedPlayerSnapshot, PersistenceError, PersistenceRuntime, PlayerStore,
};
use crate::plugins::spells::{SpellCastRequest, SpellId, SpellReleaseRequest};
use crate::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};

/// Impostazioni di connessione del server, conservate come risorsa.
/// Pubblica perché usata come marker `run_if` dai sistemi che devono girare
/// solo lato server (es. AI degli enemy).
#[derive(Resource)]
pub struct ServerConnectionConfig {
    pub server_addr: SocketAddr,
}

/// Marker applicato al link server-side dopo che il player è stato caricato e
/// spawnato, per evitare join multipli dello stesso peer.
#[derive(Component, Default)]
pub struct Joined;

/// Marker temporaneo: una query di caricamento/creazione è già in corso per il link.
#[derive(Component, Default)]
pub struct PendingJoin;

/// Identificatore del record PostgreSQL. Resta esclusivamente lato server.
#[derive(Component, Clone, Copy)]
pub struct DbPlayerId(pub Uuid);

struct JoinLoadResult {
    client_entity: Entity,
    peer_id: PeerId,
    result: Result<PersistedPlayerSnapshot, PersistenceError>,
}

/// Canale tra task Tokio e sistemi ECS. Il receiver è protetto perché `std::sync::mpsc::Receiver`
/// non è `Sync`, requisito necessario per una Bevy `Resource`.
#[derive(Resource)]
struct JoinLoadResults {
    sender: mpsc::Sender<JoinLoadResult>,
    receiver: Mutex<mpsc::Receiver<JoinLoadResult>>,
}

impl Default for JoinLoadResults {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver: Mutex::new(receiver),
        }
    }
}

pub struct ServerPlugins {
    pub server_addr: SocketAddr,
    pub tick_duration: Duration,
}

impl Plugin for ServerPlugins {
    fn build(&self, app: &mut App) {
        app.init_resource::<JoinLoadResults>();
        app.insert_resource(ServerConnectionConfig {
            server_addr: self.server_addr,
        });

        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: self.tick_duration,
        });

        app.insert_resource(ReplicationMetadata::new(Duration::from_secs_f64(
            1.0 / 60.0,
        )));

        app.add_systems(
            Startup,
            (start_server, spawn_demo_enemy, spawn_demo_dummy).chain(),
        );

        app.add_observer(handle_new_client);
        app.add_observer(handle_connected_client);
        app.add_observer(handle_disconnected_client);

        app.add_systems(
            Update,
            (
                handle_join_request,
                finish_pending_joins,
                handle_spell_cast_commands,
                handle_spell_release_commands,
                handle_respawn_requests,
                send_messages,
            )
                .chain(),
        );
    }
}

// Avvia il server: spawna l'entità server e inizia ad ascoltare.
fn start_server(config: Res<ServerConnectionConfig>, mut commands: Commands) {
    let server = commands
        .spawn((
            NetcodeServer::new(NetcodeConfig {
                client_timeout_secs: 10,
                ..default()
            }),
            LocalAddr(config.server_addr),
            ServerUdpIo::default(),
        ))
        .id();

    commands.trigger(Start { entity: server });
}

// Aggiunge il replication sender ai nuovi link client.
fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert((ReplicationSender, Name::from("Client")));
}

// Solo logging al `Connected`: il player viene spawnato dal sistema
// `handle_join_request` quando arriva il `JoinRequest` con il nome scelto.
fn handle_connected_client(trigger: On<Add, Connected>, query: Query<&RemoteId>) {
    info!(
        "handle_connected_client triggered for entity {:?}",
        trigger.entity
    );
    let Ok(client_id) = query.get(trigger.entity) else {
        info!("Failed to get RemoteId for entity {:?}", trigger.entity);
        return;
    };
    info!("Client connected: peer {:?}", client_id.0);
}

// Consuma il primo `JoinRequest` per link connesso e avvia il caricamento DB.
// Il marker `PendingJoin` rende l'operazione idempotente mentre il task async è in corso.
fn handle_join_request(
    mut receivers: Query<
        (Entity, &mut MessageReceiver<JoinRequest>, &RemoteId),
        (With<Connected>, Without<Joined>, Without<PendingJoin>),
    >,
    store: Res<PlayerStore>,
    runtime: Res<PersistenceRuntime>,
    results: Res<JoinLoadResults>,
    mut commands: Commands,
) {
    for (client_entity, mut receiver, remote_id) in receivers.iter_mut() {
        let Some(JoinRequest { player_name }) = receiver.receive().next() else {
            continue;
        };

        let Ok(display_name) = validate_player_name(&player_name) else {
            warn!("Rejecting invalid join name from peer {:?}", remote_id.0);
            commands.trigger(Disconnect {
                entity: client_entity,
            });
            continue;
        };

        let peer_id = remote_id.0;
        let normalized_name = normalize_name(&display_name);
        let repository = store.0.clone();
        let sender = results.sender.clone();

        commands.entity(client_entity).insert(PendingJoin);
        runtime.0.spawn(async move {
            let result = repository
                .find_or_create_snapshot(&normalized_name, &display_name)
                .await;
            let _ = sender.send(JoinLoadResult {
                client_entity,
                peer_id,
                result,
            });
        });
    }
}

// Riceve i risultati DB sul main thread e solo qui crea entità Bevy.
fn finish_pending_joins(
    results: Res<JoinLoadResults>,
    connected: Query<(), With<Connected>>,
    pending: Query<(), With<PendingJoin>>,
    active_players: Query<&DbPlayerId, With<Player>>,
    mut commands: Commands,
) {
    let completed: Vec<_> = {
        let receiver = results
            .receiver
            .lock()
            .expect("join result receiver poisoned");
        std::iter::from_fn(|| receiver.try_recv().ok()).collect()
    };

    for completed_join in completed {
        if connected.get(completed_join.client_entity).is_err()
            || pending.get(completed_join.client_entity).is_err()
        {
            continue;
        }

        let snapshot = match completed_join.result {
            Ok(record) => record,
            Err(error) => {
                error!(
                    "Failed to load player for {:?}: {error}",
                    completed_join.peer_id
                );
                commands.trigger(Disconnect {
                    entity: completed_join.client_entity,
                });
                continue;
            }
        };

        if active_players.iter().any(|id| id.0 == snapshot.player.id) {
            warn!(
                "Rejecting duplicate active player name {:?}",
                snapshot.player.display_name
            );
            commands.trigger(Disconnect {
                entity: completed_join.client_entity,
            });
            continue;
        }

        let peer_bits = completed_join.peer_id.to_bits();
        let hue = ((peer_bits.wrapping_mul(137).wrapping_add(180) % 360) as f32) / 360.0;
        let color = Color::hsl(hue, 0.8, 0.5);
        let position = Position(Vec3::new(
            snapshot.player.pos_x,
            snapshot.player.pos_y,
            snapshot.player.pos_z,
        ));

        let player = commands
            .spawn((
                GameEntityBundle::new(
                    position,
                    EntityColor(color),
                    snapshot.stats,
                    crate::plugins::entity::components::EntityKind::Player,
                    NetworkTarget::All,
                ),
                PlayerName(snapshot.player.display_name),
                Player::bundle(),
                PlayerId(completed_join.peer_id),
                DbPlayerId(snapshot.player.id),
                PredictionTarget::to_clients(NetworkTarget::Single(completed_join.peer_id)),
                InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(
                    completed_join.peer_id,
                )),
                ControlledBy {
                    owner: completed_join.client_entity,
                    lifetime: Default::default(),
                },
                ActionState::<Inputs>::default(),
            ))
            .id();

        commands
            .entity(completed_join.client_entity)
            .remove::<PendingJoin>()
            .insert(Joined);
        info!(
            "Created persisted player entity {player:?} for client peer {:?}",
            completed_join.peer_id
        );
    }
}

// Cattura l'ultima posizione autorevole prima di rimuovere il player server-side.
fn handle_disconnected_client(
    trigger: On<Add, Disconnected>,
    remote_ids: Query<&RemoteId>,
    players: Query<
        (
            Entity,
            &PlayerId,
            &DbPlayerId,
            &Position,
            &MovementStats,
            &CombatStats,
            &VitalStats,
        ),
        With<Player>,
    >,
    store: Res<PlayerStore>,
    runtime: Res<PersistenceRuntime>,
    mut commands: Commands,
) {
    let Ok(remote_id) = remote_ids.get(trigger.entity) else {
        return;
    };

    let Some((player_entity, _, database_id, position, movement, combat, vital)) = players
        .iter()
        .find(|(_, player_id, _, _, _, _, _)| player_id.0 == remote_id.0)
    else {
        commands
            .entity(trigger.entity)
            .remove::<Joined>()
            .remove::<PendingJoin>();
        return;
    };

    let repository = store.0.clone();
    let database_id = database_id.0;
    let position = position.0;
    let stats = StatsBundleData::from_components(movement, combat, vital);
    runtime.0.spawn(async move {
        if let Err(error) = repository
            .save_snapshot(database_id, position.x, position.y, position.z, &stats)
            .await
        {
            error!("Failed to save player position {database_id}: {error}");
        }
    });

    commands.entity(player_entity).despawn();
    commands
        .entity(trigger.entity)
        .remove::<Joined>()
        .remove::<PendingJoin>();
}

/// Spawn di un enemy demo all'avvio del server.
/// `spawn_entity::<Enemy>()` applica automaticamente `GameEntity`, statistiche,
/// `Position`, `EntityColor`, il bundle di `Enemy` e `Replicate`. La posizione
/// e il colore di default sono dichiarati in `<Enemy as EntityDefinition>`.
fn spawn_demo_enemy(mut commands: Commands) {
    let enemy = spawn_entity::<Enemy>(&mut commands);
    info!("Spawned demo enemy {:?}", enemy);
}

fn spawn_demo_dummy(mut commands: Commands) {
    let dummy = spawn_entity::<Dummy>(&mut commands);
    info!("Spawned demo dummy {:?}", dummy);
}

/// Traduce i comandi spell arrivati dal network in richieste ECS interne.
///
/// Il client non può indicare direttamente l'entità caster: il server la risolve
/// dal peer connesso e dal `PlayerId` del player già joinato.
fn handle_spell_cast_commands(
    mut receivers: Query<
        (&mut MessageReceiver<SpellCastCommand>, &RemoteId),
        (With<Connected>, With<Joined>),
    >,
    players: Query<(Entity, &PlayerId), With<Player>>,
    targets: Query<
        (Entity, &NetworkEntityId),
        With<crate::plugins::entity::components::GameEntity>,
    >,
    mut spell_cast_requests: MessageWriter<SpellCastRequest>,
) {
    for (mut receiver, remote_id) in receivers.iter_mut() {
        let Some((player_entity, _)) = players
            .iter()
            .find(|(_, player_id)| player_id.0 == remote_id.0)
        else {
            continue;
        };

        for command in receiver.receive() {
            let target_entity = command.target_id.and_then(|target_id| {
                targets
                    .iter()
                    .find(|(_, network_id)| network_id.0 == target_id)
                    .map(|(entity, _)| entity)
            });

            if command.target_id.is_some() && target_entity.is_none() {
                bevy::log::warn!(
                    "Player {:?} cast {} with unknown target_id {:?}",
                    player_entity,
                    command.spell_id,
                    command.target_id
                );
            }

            bevy::log::info!(
                "Received spell cast command from player {:?}: spell={}, target_id={:?}, resolved_target={:?}",
                player_entity,
                command.spell_id,
                command.target_id,
                target_entity
            );
            spell_cast_requests.write(SpellCastRequest {
                caster: player_entity,
                spell_id: SpellId::new(command.spell_id.clone()),
                target_position: command.target_position,
                target_entity,
            });
        }
    }
}

/// Traduce i comandi di rilascio spell (`SpellCastRelease`) in eventi interni
/// [`SpellReleaseRequest`] che il sistema di spells consuma per terminare
/// channeling o cancellare cast-time.
fn handle_spell_release_commands(
    mut receivers: Query<
        (&mut MessageReceiver<SpellCastRelease>, &RemoteId),
        (With<Connected>, With<Joined>),
    >,
    players: Query<(Entity, &PlayerId), With<Player>>,
    mut release_requests: MessageWriter<SpellReleaseRequest>,
) {
    for (mut receiver, remote_id) in receivers.iter_mut() {
        let Some((player_entity, _)) = players
            .iter()
            .find(|(_, player_id)| player_id.0 == remote_id.0)
        else {
            continue;
        };

        for command in receiver.receive() {
            release_requests.write(SpellReleaseRequest {
                caster: player_entity,
                spell_id: SpellId::new(command.spell_id.clone()),
            });
        }
    }
}

/// Processa le richieste di respawn: solo i player `Dead` vengono riportati
/// in vita, gli altri vengono ignorati (no-op silenzioso). La mutazione è
/// server-authoritative e viene replicata ai client tramite `Position`,
/// `VitalStats` ed `EntityState`.
fn handle_respawn_requests(
    mut receivers: Query<
        (&mut MessageReceiver<RespawnRequest>, &RemoteId),
        (With<Connected>, With<Joined>),
    >,
    mut players: Query<
        (
            Entity,
            &PlayerId,
            &mut Position,
            &mut VitalStats,
            &mut EntityState,
            Option<&SpawnPoint>,
        ),
        With<Player>,
    >,
    mut respawned: MessageWriter<RespawnedEvent>,
) {
    for (mut receiver, remote_id) in receivers.iter_mut() {
        // `RespawnRequest` non ha payload: ci basta consumerlo se presente.
        let mut requested = false;
        for _ in receiver.receive() {
            requested = true;
        }
        if !requested {
            continue;
        }

        for (player_entity, player_id, mut position, mut vital, mut state, spawn_point) in
            players.iter_mut()
        {
            if player_id.0 != remote_id.0 {
                continue;
            }
            if *state != EntityState::Dead {
                // Player ancora vivo: ignora.
                continue;
            }

            let target_position = spawn_point.map(|s| s.0).unwrap_or(PLAYER_SPAWN_POINT);
            position.0 = target_position;
            vital.current_health = vital.max_health;
            vital.clamp_health();
            *state = EntityState::Idle;
            respawned.write(RespawnedEvent {
                entity: player_entity,
            });
            info!(
                "Player {:?} respawned at {:?}",
                player_entity, target_position
            );
            break;
        }
    }
}

// Invia messaggi ai client.
fn send_messages(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut sender: ServerMultiMessageSender,
    server: Single<&lightyear::prelude::server::Server>,
) {
    let Some(keys) = keys else {
        return;
    };

    if keys.just_pressed(KeyCode::KeyM) {
        let message = PlayerMessage(42);
        info!("Sending message: {:?}", message);
        sender
            .send::<PlayerMessage, Channel1>(&message, server.into_inner(), &NetworkTarget::All)
            .unwrap_or_else(|e| {
                error!("Failed to send message: {:?}", e);
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_valid_display_name_has_a_stable_database_key() {
        let display_name = validate_player_name("  Ada  ").expect("valid name");
        assert_eq!(display_name, "Ada");
        assert_eq!(normalize_name(&display_name), "ada");
    }

    #[test]
    fn invalid_names_are_rejected_before_database_access() {
        assert!(validate_player_name("ab").is_err());
        assert!(validate_player_name("").is_err());
    }
}
