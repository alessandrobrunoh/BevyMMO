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

use bevymmo_shared::entity::boss::components::Boss;
use bevymmo_shared::entity::components::{EntityKind, EntityState, PlayerName, SpawnPoint};
use bevymmo_shared::entity::dummy::components::Dummy;
use bevymmo_shared::entity::enemy::components::Enemy;
use bevymmo_shared::entity::events::RespawnedEvent;
use bevymmo_shared::entity::player::components::Player;
use bevymmo_shared::entity::player::spawn::PLAYER_SPAWN_POINT;
use bevymmo_shared::entity::spawn::{spawn_entity, GameEntityBundle};
use bevymmo_shared::game_state::validate_player_name;
use bevymmo_shared::network::protocol::*;
use bevymmo_shared::spells::{
    SpellCastRequest, SpellCooldowns, SpellHotbar, SpellId, SpellRegistry, SpellReleaseRequest,
};
use bevymmo_shared::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};

use crate::persistence::{
    normalize_name, PersistedPlayerSnapshot, PersistenceError, PersistenceRuntime, PlayerStore,
};

/// Server connection settings, stored as a resource.
/// Public because used as a `run_if` marker by systems that should only
/// run server-side (e.g. enemy AI).
#[derive(Resource)]
pub struct ServerConnectionConfig {
    pub server_addr: SocketAddr,
}

/// Marker applied to the server-side link after the player is loaded and
/// spawned, to prevent multiple joins from the same peer.
#[derive(Component, Default)]
pub struct Joined;

/// Temporary marker: a load/create query is already in progress for the link.
#[derive(Component, Default)]
pub struct PendingJoin;

/// PostgreSQL record identifier. Remains strictly server-side.
#[derive(Component, Clone, Copy)]
pub struct DbPlayerId(pub Uuid);

struct JoinLoadResult {
    client_entity: Entity,
    peer_id: PeerId,
    result: Result<PersistedPlayerSnapshot, PersistenceError>,
}

/// Channel between Tokio tasks and ECS systems. The receiver is protected because `std::sync::mpsc::Receiver`
/// is not `Sync`, which is required for a Bevy `Resource`.
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
            (start_server, spawn_demo_enemy, spawn_demo_dummy, spawn_boss).chain(),
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
                handle_update_hotbar_slot_requests,
                handle_respawn_requests,
                send_messages,
            )
                .chain(),
        );
    }
}

// Starts the server: spawns the server entity and begins listening.
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

// Adds the replication sender to new client links.
fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert((ReplicationSender, Name::from("Client")));
}

// Logging only on `Connected`: the player is spawned by the
// `handle_join_request` system when `JoinRequest` arrives with the chosen name.
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

// Consumes the first `JoinRequest` for a connected link and starts DB loading.
// The `PendingJoin` marker makes the operation idempotent while the async task is in progress.
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

// Receives DB results on the main thread and creates Bevy entities only here.
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
                    EntityKind::Player,
                    NetworkTarget::All,
                ),
                PlayerName(snapshot.player.display_name),
                Player,
                snapshot.hotbar,
                SpellCooldowns::default(),
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

// Captures the last authoritative position before removing the player server-side.
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
            &SpellHotbar,
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

    let Some((player_entity, _, database_id, position, movement, combat, vital, hotbar)) = players
        .iter()
        .find(|(_, player_id, _, _, _, _, _, _)| player_id.0 == remote_id.0)
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
    let hotbar = hotbar.clone();
    runtime.0.spawn(async move {
        if let Err(error) = repository
            .save_snapshot(
                database_id,
                position.x,
                position.y,
                position.z,
                &stats,
                &hotbar,
            )
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

/// Spawns a demo enemy at server startup.
/// `spawn_entity::<Enemy>()` automatically applies `GameEntity`, stats,
/// `Position`, `EntityColor`, the `Enemy` bundle, and `Replicate`. Default position
/// and color are declared in `<Enemy as EntityDefinition>`.
fn spawn_demo_enemy(mut commands: Commands) {
    let enemy = spawn_entity::<Enemy>(&mut commands);
    info!("Spawned demo enemy {:?}", enemy);
}

fn spawn_demo_dummy(mut commands: Commands) {
    let dummy = spawn_entity::<Dummy>(&mut commands);
    info!("Spawned demo dummy {:?}", dummy);
}

/// Spawns the dragon boss at server startup.
///
/// `spawn_entity::<Boss>()` applies `GameEntity`, stats, `Position`,
/// `EntityColor`, the boss bundle (`Boss`, `BossPhase::Dormant`, `BossArena`,
/// threat/spellbook/rotation state) and `Replicate`. The boss starts dormant
/// until a player enters the arena ring (handled in Phase 1).
fn spawn_boss(mut commands: Commands) {
    let boss = spawn_entity::<Boss>(&mut commands);
    info!("Spawned boss {:?}", boss);
}

/// Translates spell commands arriving from the network into internal ECS requests.
///
/// The client cannot directly specify the caster entity: the server resolves it
/// from the connected peer and the `PlayerId` of the already joined player.
fn handle_spell_cast_commands(
    mut receivers: Query<
        (&mut MessageReceiver<SpellCastCommand>, &RemoteId),
        (With<Connected>, With<Joined>),
    >,
    players: Query<(Entity, &PlayerId), With<Player>>,
    targets: Query<
        (Entity, &NetworkEntityId),
        With<bevymmo_shared::entity::components::GameEntity>,
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

            bevy::log::debug!(
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

/// Translates spell release commands (`SpellCastRelease`) into internal [`SpellReleaseRequest`]
/// events that the spell system consumes to terminate channeling or cancel cast-time.
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

/// Processes respawn requests: only `Dead` players are brought back
/// to life, others are ignored (silent no-op). The mutation is
/// server-authoritative and replicated to clients via `Position`,
/// `VitalStats`, and `EntityState`.
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
        // `RespawnRequest` has no payload: consuming it if present is sufficient.
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
            if *state != EntityState::Dead && !vital.is_dead() {
                // Player still alive: ignore. Health zero is also considered
                // dead while the replicated EntityState catches up.
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

// Sends messages to clients.
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

#[allow(clippy::type_complexity)]
fn handle_update_hotbar_slot_requests(
    mut receivers: Query<
        (&mut MessageReceiver<UpdateHotbarSlotRequest>, &RemoteId),
        (With<Connected>, With<Joined>),
    >,
    mut players: Query<(Entity, &PlayerId, &DbPlayerId, &mut SpellHotbar), With<Player>>,
    registry: Res<SpellRegistry>,
    store: Res<PlayerStore>,
    runtime: Res<PersistenceRuntime>,
) {
    for (mut receiver, remote_id) in receivers.iter_mut() {
        let Some((player_entity, _, database_id, mut hotbar)) = players
            .iter_mut()
            .find(|(_, player_id, _, _)| player_id.0 == remote_id.0)
        else {
            continue;
        };

        for request in receiver.receive() {
            let spell_id = request.spell_id.map(SpellId::new);
            if let Some(id) = &spell_id {
                if !registry.contains(id) {
                    bevy::log::warn!(
                        "Player {:?} attempted to assign unknown spell {}",
                        player_entity,
                        id.as_str()
                    );
                    continue;
                }
            }

            hotbar.assign(request.slot, spell_id);

            let repository = store.0.clone();
            let db_id = database_id.0;
            let current_hotbar = hotbar.clone();
            runtime.0.spawn(async move {
                if let Err(e) = repository.save_hotbar(db_id, &current_hotbar).await {
                    bevy::log::error!("Failed to save hotbar for {}: {}", db_id, e);
                }
            });
        }
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
