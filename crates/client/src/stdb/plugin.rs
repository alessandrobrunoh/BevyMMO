//! Connection lifecycle and row-to-entity mirroring.
//!
//! # How state gets from the database into the ECS
//!
//! The SDK delivers rows through callbacks that run on whichever thread calls
//! [`DbConnection::frame_tick`], and those callbacks cannot borrow the Bevy
//! `World`. So they do the least possible work — clone the row into a
//! [`crossbeam_channel`] — and [`drain_events`] applies them from a normal
//! system, where mutating the world is safe.
//!
//! The components written out are the *same ones lightyear replicated*
//! (`Position`, `VitalStats`, `Inventory`, ...), which is what keeps
//! `bevymmo_presentation` from noticing the change of transport at all.
//!
//! # Why the client simulates too
//!
//! lightyear gave prediction and interpolation for free; SpacetimeDB gives
//! neither. The server ticks at roughly 18-19 Hz, so rendering raw authoritative
//! positions would visibly stutter. Instead every entity carries its destination
//! ([`StdbAuthoritative::move_target`], replicated on purpose), and the client
//! walks towards it every frame using [`bevymmo_domain::movement::step_towards`].
//! The server additionally resolves terrain and collision from its embedded
//! world data, so reconciliation remains responsible for correcting local
//! prediction around slopes and blockers.

use bevy::prelude::*;
use bevymmo_domain::movement::{self, Step};
use bevymmo_domain::spells::components::SpellHotbar;
use bevymmo_domain::spells::registry::SpellId;
use bevymmo_domain::stats::events::{ModifierKind, ModifierOp, StatField};
use bevymmo_domain::stats::modifiers::{
    ActiveStatModifiers, ModifierEffectInstance, ModifierId as StatModifierId, StatModifierInstance,
};
use bevymmo_domain::EntityId;
use bevymmo_shared::abilities::{AncientWordId, EssenceId, KnownGlyphs, ModifierId};
use bevymmo_shared::crowd_control::{ActiveCrowdControl, CrowdControlKind, CrowdControlState};
use bevymmo_shared::entity::boss::components::{Boss, BossArena, BossPhase};
use bevymmo_shared::entity::components::{EntityKind, EntityState, GameEntity, PlayerName};
use bevymmo_shared::entity::LocalPlayer;
use bevymmo_shared::game_state::{
    ConnectionFailure, ConnectionIntent, ConnectionRequest, GameScreen, Screen,
};
use bevymmo_shared::items::components::{Equipment, Inventory};
use bevymmo_shared::movement::{resolve_ray_to_ground, ClientSurfaceQuery};
use bevymmo_shared::network::protocol::{SpellCastEnded, SpellCastProgress, SpellVisualEffect};
use bevymmo_shared::server_feed::{ServerNotice, SpellCooldownState};
use bevymmo_shared::stats::components::{CombatStats, MovementStats, VitalStats};
use bevymmo_shared::world_components::{
    EntityColor, LookDirection, NetworkEntityId, Position, ProjectileVisual,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use spacetimedb_sdk::{credentials, DbContext, EventTable, Identity, Table, TableWithPrimaryKey};
use std::collections::{HashMap, HashSet};

use super::module_bindings::boss_state_table::BossStateTableAccess;
use super::module_bindings::cast_ended_table::CastEndedTableAccess;
use super::module_bindings::cast_state_table::CastStateTableAccess;
use super::module_bindings::cooldown_table::CooldownTableAccess;
use super::module_bindings::crowd_control_table::CrowdControlTableAccess;
use super::module_bindings::entity_stats_table::EntityStatsTableAccess;
use super::module_bindings::equipment_table::EquipmentTableAccess;
use super::module_bindings::game_entity_table::GameEntityTableAccess;
use super::module_bindings::heartbeat_reducer::heartbeat;
use super::module_bindings::hotbar_table::HotbarTableAccess;
use super::module_bindings::inventory_table::InventoryTableAccess;
use super::module_bindings::join_reducer::join;
use super::module_bindings::known_glyphs_table::KnownGlyphsTableAccess;
use super::module_bindings::move_to_reducer::move_to;
use super::module_bindings::periodic_effect_table::PeriodicEffectTableAccess;
use super::module_bindings::player_message_table::PlayerMessageTableAccess;
use super::module_bindings::player_table::PlayerTableAccess;
use super::module_bindings::projectile_table::ProjectileTableAccess;
use super::module_bindings::spell_visual_effect_table::SpellVisualEffectTableAccess;
use super::module_bindings::stat_modifier_table::StatModifierTableAccess;
use super::module_bindings::{
    BossPhaseRow, BossState, CastEndedEvent, CastKindRow, CastState, ColorRow, Cooldown,
    CrowdControl, CrowdControlKindRow, DbConnection, EntityKindRow, EntityStateRow, EntityStats,
    EquipmentTable, GameEntity as EntityRow, Hotbar, InventoryTable, ItemInstanceRow,
    KnownGlyphsTable, ModifierKindRow, PeriodicEffect, Player, PlayerMessageEvent, Projectile,
    ReducerEventContext, RemoteReducers, SpellVisualEffectEvent, StatModifier, Vec3Row,
};

/// How fast predicted position is pulled back towards the authoritative one, as
/// a rate per second. Higher snaps harder and shows correction jitter; lower
/// drifts visibly before catching up.
const RECONCILE_RATE: f32 = 8.0;

/// Beyond this much error, stop easing and just teleport. Covers a genuine
/// desync — a teleport, a respawn, a long stall — where smoothing would send the
/// character gliding across the map.
const SNAP_DISTANCE: f32 = 5.0;

/// Seconds between destination updates while the mouse button is held.
const MOVE_COMMAND_INTERVAL: f32 = 0.1;

/// Seconds between presence heartbeats.
///
/// Comfortably inside the module's `PRESENCE_TIMEOUT_SECONDS` so an ordinary
/// stall does not read as a disconnect. The module cannot enumerate live
/// connections, so this is how it knows anyone is still here — and why a
/// restarted server settles to "nobody online" instead of showing ghosts.
const HEARTBEAT_INTERVAL: f32 = 5.0;

/// The server's last word on an entity, kept apart from the rendered
/// [`Position`] so prediction has something to reconcile against.
#[derive(Component, Debug, Clone, Copy)]
pub struct StdbAuthoritative {
    pub position: Vec3,
    pub move_target: Option<Vec3>,
    pub speed: f32,
}

/// Row changes handed from the SDK's thread to the Bevy schedule.
enum RowEvent {
    Entity(EntityRow),
    EntityRemoved(u64),
    Stats(EntityStats),
    Player(Player),
    Inventory(InventoryTable),
    Equipment(EquipmentTable),
    Hotbar(Hotbar),
    KnownGlyphs(KnownGlyphsTable),
    CastState(CastState),
    CastEnded(CastEndedEvent),
    SpellVisualEffect(SpellVisualEffectEvent),
    BossState(BossState),
    CrowdControl(CrowdControl),
    CrowdControlRemoved(CrowdControl),
    StatModifier(StatModifier),
    StatModifierRemoved(StatModifier),
    PeriodicEffect(PeriodicEffect),
    PeriodicEffectRemoved(PeriodicEffect),
    Cooldown(Cooldown),
    CooldownRemoved(Cooldown),
    Projectile(Projectile),
    ProjectileRemoved(u64),
    PlayerMessage(PlayerMessageEvent),
    /// A reducer the client called came back with the module's own `Err`.
    ReducerRejected(String),
    JoinRejected(String),
}

/// Latest rows retained until the dependent Bevy entity exists. Initial
/// subscription rows have no delivery order guarantee.
///
/// Caches the latest received rows per entity so that `replay_entity` can
/// restore full component state on (re)connect or after a reconcile gap.
#[derive(Resource, Default)]
struct PendingRows {
    entities: HashMap<u64, EntityRow>,
    offline_players: HashSet<Identity>,
    stats: HashMap<u64, EntityStats>,
    inventory: HashMap<Identity, InventoryTable>,
    equipment: HashMap<Identity, EquipmentTable>,
    hotbar: HashMap<Identity, Hotbar>,
    known_glyphs: HashMap<Identity, KnownGlyphsTable>,
    boss_state: HashMap<u64, BossState>,
    /// Keyed by `crowd_control.id`, not by entity: one entity can carry several.
    crowd_control: HashMap<u64, CrowdControl>,
    /// Keyed by `stat_modifier.id`.
    stat_modifier: HashMap<u64, StatModifier>,
    /// Keyed by `periodic_effect.id`.
    periodic_effect: HashMap<u64, PeriodicEffect>,
    /// What was last written to each entity's `CrowdControlState` and
    /// `ActiveStatModifiers`.
    ///
    /// Both are rebuilt from scratch whenever any contributing row changes, and
    /// `replay_entity` runs on every `game_entity` update — which is every
    /// moving entity, twenty times a second. Without this an unchanged, empty
    /// state was re-inserted on each of those, so every `Changed<>` filter in
    /// the UI fired continuously on entities that had no effects at all.
    applied: HashMap<u64, AppliedEffects>,
}

/// The last effect state written to one entity, for change suppression.
#[derive(Default, PartialEq, Debug)]
struct AppliedEffects {
    crowd_control: CrowdControlState,
    /// The contributing rows, as `(id, remaining_seconds bits)`.
    /// `StatModifierInstance` has no `PartialEq` to compare the built component
    /// with, and the rows are what decides it anyway.
    modifier_signature: Vec<(u64, u32)>,
}

/// Owns the connection. Call reducers through [`StdbConnection::reducers`].
#[derive(Resource)]
pub struct StdbConnection {
    conn: DbConnection,
    events: Receiver<RowEvent>,
    /// Handed to reducer callbacks so a server-side refusal can travel the same
    /// path as a row change, and reach the schedule where it can be shown.
    reports: Sender<RowEvent>,
}

#[derive(Resource, Clone)]
struct StdbConnectionConfig {
    uri: String,
    module: String,
}

impl StdbConnection {
    /// The reducer handle — how the client asks the server to do anything.
    pub fn reducers(&self) -> &RemoteReducers {
        self.conn.reducers()
    }

    /// This client's identity, once the connection has been established.
    pub fn identity(&self) -> Option<Identity> {
        self.conn.try_identity()
    }

    /// Builds the callback every reducer wrapper hands to its `*_then` form.
    ///
    /// The module answers a rejected call with a sentence written for the
    /// player — "inventory is full", "target is out of range", "that name is
    /// taken". Before this existed every one of them was discarded: the
    /// fire-and-forget send reported only whether the *request* left the
    /// machine, never what the server made of it.
    ///
    /// `action` names what the player was trying to do, so the notice reads as
    /// a sentence rather than as a bare server string.
    pub(super) fn report_rejection(
        &self,
        action: &'static str,
    ) -> impl FnOnce(&ReducerEventContext, ReducerOutcome) + Send + 'static {
        let reports = self.reports.clone();
        move |_ctx, outcome| {
            let message = match outcome {
                Ok(Ok(())) => return,
                Ok(Err(reason)) => format!("{action}: {reason}"),
                // The call reached the module but its result could not be
                // decoded. Not the player's fault, but silence would be worse.
                Err(err) => format!("{action}: {err}"),
            };
            let _ = reports.send(RowEvent::ReducerRejected(message));
        }
    }

    fn report_join_rejection(
        &self,
    ) -> impl FnOnce(&ReducerEventContext, ReducerOutcome) + Send + 'static {
        let reports = self.reports.clone();
        move |_ctx, outcome| {
            let message = match outcome {
                Ok(Ok(())) => return,
                Ok(Err(reason)) => format!("Impossibile entrare: {reason}"),
                Err(err) => format!("Impossibile entrare: {err}"),
            };
            let _ = reports.send(RowEvent::JoinRejected(message));
        }
    }
}

/// What a `*_then` callback is handed: the module's own `Result`, or the SDK
/// failing to decode one.
type ReducerOutcome =
    Result<Result<(), String>, spacetimedb_sdk::__codegen::InternalError>;

/// Maps server entity ids to the Bevy entities mirroring them.
#[derive(Resource, Default)]
pub struct StdbEntityMap {
    by_entity_id: HashMap<u64, Entity>,
    /// Which server entity belongs to which account, so the per-character
    /// tables (inventory, equipment, hotbar) can find the entity to attach to.
    entity_of_identity: HashMap<Identity, u64>,
    /// Projectiles live in their own id space — `projectile.id` counts
    /// separately from `game_entity.entity_id` — so they get their own map
    /// rather than colliding in the one above.
    projectiles: HashMap<u64, Entity>,
}

impl StdbEntityMap {
    pub fn get(&self, entity_id: u64) -> Option<Entity> {
        self.by_entity_id.get(&entity_id).copied()
    }
}

pub struct StdbPlugin {
    pub uri: String,
    pub module: String,
}

impl Plugin for StdbPlugin {
    fn build(&self, app: &mut App) {
        let uri = self.uri.clone();
        let module = self.module.clone();

        app.init_resource::<StdbEntityMap>();
        app.init_resource::<PendingRows>();
        app.insert_resource(StdbConnectionConfig {
            uri: uri.clone(),
            module: module.clone(),
        });
        app.add_systems(Startup, move |world: &mut World| {
            match connect(&uri, &module) {
                Ok(connection) => world.insert_resource(connection),
                // Not fatal: the menu stays usable and the player can retry
                // rather than the process dying on a cold database.
                Err(err) => error!("SpacetimeDB connection to {uri} failed: {err}"),
            }
        });
        app.add_systems(
            PreUpdate,
            (pump_connection, drain_events)
                .chain()
                .run_if(resource_exists::<StdbConnection>),
        );
        app.add_systems(
            Update,
            (join_on_request, send_move_commands, send_heartbeat)
                .run_if(resource_exists::<StdbConnection>),
        );
        app.add_systems(Update, predict_and_reconcile);
    }
}

/// Where the auth token is cached between runs, so a returning player keeps
/// their character. Without it every launch is a brand-new `Identity` — and
/// since the character is keyed by identity, a brand-new character.
fn credential_store() -> credentials::File {
    credentials::File::new("bevymmo")
}

fn clear_cached_token() {
    let Some(home) = dirs::home_dir() else {
        warn!("could not determine the home directory to clear the SpacetimeDB token");
        return;
    };
    let path = home.join(".spacetimedb_client_credentials/bevymmo");
    match std::fs::remove_file(&path) {
        Ok(()) => info!("cleared cached SpacetimeDB identity"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => warn!(
            "could not clear cached SpacetimeDB token at {}: {err}",
            path.display()
        ),
    }
}

fn connect(uri: &str, module: &str) -> Result<StdbConnection, Box<dyn std::error::Error>> {
    let (tx, events) = unbounded();
    let reports = tx.clone();

    let conn = DbConnection::builder()
        .with_uri(uri)
        .with_database_name(module)
        .with_token(credential_store().load().ok().flatten())
        .on_connect(|_ctx, _identity, token| {
            if let Err(err) = credential_store().save(token) {
                warn!("could not cache the SpacetimeDB token: {err}");
            }
        })
        .on_connect_error(|_ctx, err| error!("SpacetimeDB connection error: {err}"))
        .on_disconnect(|_ctx, err| match err {
            Some(err) => error!("disconnected from SpacetimeDB: {err}"),
            None => info!("disconnected from SpacetimeDB"),
        })
        .build()?;

    register_callbacks(&conn, tx);

    conn.subscription_builder()
        .on_applied(|_ctx| info!("SpacetimeDB subscription applied"))
        .on_error(|_ctx, err| error!("SpacetimeDB subscription failed: {err}"))
        .subscribe([
            "SELECT * FROM game_entity",
            "SELECT * FROM entity_stats",
            "SELECT * FROM player",
            "SELECT * FROM inventory",
            "SELECT * FROM equipment",
            "SELECT * FROM hotbar",
            "SELECT * FROM known_glyphs",
            "SELECT * FROM cast_state",
            "SELECT * FROM boss_state",
            "SELECT * FROM crowd_control",
            "SELECT * FROM stat_modifier",
            "SELECT * FROM periodic_effect",
            "SELECT * FROM cooldown",
            "SELECT * FROM projectile",
            "SELECT * FROM cast_ended",
            "SELECT * FROM spell_visual_effect",
            "SELECT * FROM player_message",
        ]);

    Ok(StdbConnection {
        conn,
        events,
        reports,
    })
}

/// Every callback does the same thing: clone the row onto the channel. They stay
/// this dumb on purpose — they run outside the Bevy schedule and must not touch
/// the world.
fn register_callbacks(conn: &DbConnection, tx: Sender<RowEvent>) {
    macro_rules! mirror {
        ($table:ident, $variant:ident) => {{
            let inserted = tx.clone();
            conn.db().$table().on_insert(move |_ctx, row| {
                let _ = inserted.send(RowEvent::$variant(row.clone()));
            });
            let updated = tx.clone();
            conn.db().$table().on_update(move |_ctx, _old, new| {
                let _ = updated.send(RowEvent::$variant(new.clone()));
            });
        }};
    }

    mirror!(game_entity, Entity);
    mirror!(entity_stats, Stats);
    mirror!(player, Player);
    mirror!(inventory, Inventory);
    mirror!(equipment, Equipment);
    mirror!(hotbar, Hotbar);
    mirror!(known_glyphs, KnownGlyphs);
    mirror!(cast_state, CastState);
    mirror!(boss_state, BossState);
    mirror!(crowd_control, CrowdControl);
    mirror!(stat_modifier, StatModifier);
    mirror!(periodic_effect, PeriodicEffect);
    mirror!(cooldown, Cooldown);
    mirror!(projectile, Projectile);

    // Deletions matter for anything the client keeps a copy of: a stun that
    // ends, a buff that expires, a projectile that lands. Without these the
    // effect would stay on screen with a frozen timer.
    macro_rules! mirror_delete {
        ($table:ident, $variant:ident) => {{
            let deleted = tx.clone();
            conn.db().$table().on_delete(move |_ctx, row| {
                let _ = deleted.send(RowEvent::$variant(row.clone()));
            });
        }};
    }

    mirror_delete!(crowd_control, CrowdControlRemoved);
    mirror_delete!(stat_modifier, StatModifierRemoved);
    mirror_delete!(periodic_effect, PeriodicEffectRemoved);
    mirror_delete!(cooldown, CooldownRemoved);

    // Event tables are insert-only by design: a row is delivered and not
    // retained, which is exactly the lifetime "play this once" wants.
    macro_rules! mirror_event {
        ($table:ident, $variant:ident) => {{
            let fired = tx.clone();
            conn.db().$table().on_insert(move |_ctx, row| {
                let _ = fired.send(RowEvent::$variant(row.clone()));
            });
        }};
    }

    mirror_event!(cast_ended, CastEnded);
    mirror_event!(spell_visual_effect, SpellVisualEffect);
    mirror_event!(player_message, PlayerMessage);

    let projectile_removed = tx.clone();
    conn.db().projectile().on_delete(move |_ctx, row| {
        let _ = projectile_removed.send(RowEvent::ProjectileRemoved(row.id));
    });

    let removed = tx.clone();
    conn.db().game_entity().on_delete(move |_ctx, row| {
        let _ = removed.send(RowEvent::EntityRemoved(row.entity_id));
    });
}

/// Processes whatever the server has sent since the last frame.
///
/// `frame_tick` is the non-blocking variant: it applies pending messages and
/// returns rather than owning a thread. That keeps every row callback on the
/// main thread and inside the Bevy frame, which is why [`drain_events`] can run
/// immediately after and see a consistent batch.
fn pump_connection(conn: Res<StdbConnection>) {
    if let Err(err) = conn.conn.frame_tick() {
        error!("SpacetimeDB frame_tick failed: {err}");
    }
}

fn drain_events(
    conn: Res<StdbConnection>,
    mut map: ResMut<StdbEntityMap>,
    mut pending: ResMut<PendingRows>,
    mut commands: Commands,
    mut cast_progress: MessageWriter<SpellCastProgress>,
    mut cast_ended: MessageWriter<SpellCastEnded>,
    mut visual_effects: MessageWriter<SpellVisualEffect>,
    mut cooldowns: MessageWriter<SpellCooldownState>,
    mut notices: MessageWriter<ServerNotice>,
    mut failure: ResMut<ConnectionFailure>,
    mut screen: ResMut<GameScreen>,
) {
    let local_identity = conn.identity();

    while let Ok(event) = conn.events.try_recv() {
        match event {
            RowEvent::Entity(row) => {
                let entity_id = row.entity_id;
                let owner = row.owner;
                pending.entities.insert(entity_id, row.clone());
                if owner.is_some_and(|identity| pending.offline_players.contains(&identity)) {
                    continue;
                }

                apply_entity(&mut commands, &mut map, &row, local_identity);
                replay_entity(&mut commands, &map, &mut pending, entity_id);
                if let Some(identity) = owner {
                    replay_identity(&mut commands, &map, &pending, identity, local_identity);
                }
            }
            RowEvent::EntityRemoved(entity_id) => {
                if let Some(entity) = map.by_entity_id.remove(&entity_id) {
                    commands.entity(entity).despawn();
                }
                // Everything keyed by this entity goes with it — including the
                // per-account rows, which are keyed by `Identity` and so were
                // outliving the character they belonged to.
                let owners: Vec<Identity> = map
                    .entity_of_identity
                    .iter()
                    .filter(|(_, id)| **id == entity_id)
                    .map(|(identity, _)| *identity)
                    .collect();
                for identity in owners {
                    map.entity_of_identity.remove(&identity);
                    pending.inventory.remove(&identity);
                    pending.equipment.remove(&identity);
                    pending.hotbar.remove(&identity);
                    pending.known_glyphs.remove(&identity);
                }
                pending.entities.remove(&entity_id);
                pending.stats.remove(&entity_id);
                pending.boss_state.remove(&entity_id);
                pending.applied.remove(&entity_id);
                pending.crowd_control.retain(|_, cc| cc.entity_id != entity_id);
                pending
                    .stat_modifier
                    .retain(|_, row| row.entity_id != entity_id);
                pending
                    .periodic_effect
                    .retain(|_, row| row.entity_id != entity_id);
            }
            RowEvent::Stats(row) => {
                let entity_id = row.entity_id;
                pending.stats.insert(entity_id, row);
                replay_entity(&mut commands, &map, &mut pending, entity_id);
            }
            RowEvent::Player(row) => {
                map.entity_of_identity.insert(row.identity, row.entity_id);
                if row.online {
                    pending.offline_players.remove(&row.identity);
                    if let Some(entity_row) = pending.entities.get(&row.entity_id).cloned() {
                        apply_entity(&mut commands, &mut map, &entity_row, local_identity);
                        replay_entity(&mut commands, &map, &mut pending, row.entity_id);
                    }
                    replay_identity(&mut commands, &map, &pending, row.identity, local_identity);
                } else {
                    pending.offline_players.insert(row.identity);
                    if let Some(entity) = map.by_entity_id.remove(&row.entity_id) {
                        commands.entity(entity).despawn();
                    }
                }
            }
            RowEvent::Inventory(row) => {
                let identity = row.identity;
                pending.inventory.insert(identity, row);
                replay_identity(&mut commands, &map, &pending, identity, local_identity);
            }
            RowEvent::Equipment(row) => {
                let identity = row.identity;
                pending.equipment.insert(identity, row);
                replay_identity(&mut commands, &map, &pending, identity, local_identity);
            }
            RowEvent::Hotbar(row) => {
                let identity = row.identity;
                pending.hotbar.insert(identity, row);
                replay_identity(&mut commands, &map, &pending, identity, local_identity);
            }
            RowEvent::KnownGlyphs(row) => {
                let identity = row.identity;
                pending.known_glyphs.insert(identity, row);
                if local_identity == Some(identity) {
                    replay_identity(&mut commands, &map, &pending, identity, local_identity);
                }
            }
            RowEvent::CastState(row) => {
                cast_progress.write(cast_progress_from(&row));
            }
            RowEvent::CastEnded(row) => {
                cast_ended.write(SpellCastEnded {
                    caster_network_id: row.entity_id,
                    spell_id: row.spell_id,
                    completed: !row.interrupted,
                });
            }
            RowEvent::SpellVisualEffect(row) => {
                visual_effects.write(SpellVisualEffect {
                    spell_id: row.spell_id,
                    start: to_vec3(&row.start),
                    end: to_vec3(&row.end),
                });
            }
            RowEvent::BossState(row) => {
                let entity_id = row.entity_id;
                pending.boss_state.insert(entity_id, row);
                replay_entity(&mut commands, &map, &mut pending, entity_id);
            }
            RowEvent::CrowdControl(row) => {
                let entity_id = row.entity_id;
                pending.crowd_control.insert(row.id, row);
                replay_entity(&mut commands, &map, &mut pending, entity_id);
            }
            RowEvent::CrowdControlRemoved(row) => {
                pending.crowd_control.remove(&row.id);
                replay_entity(&mut commands, &map, &mut pending, row.entity_id);
            }
            RowEvent::StatModifier(row) => {
                let entity_id = row.entity_id;
                pending.stat_modifier.insert(row.id, row);
                replay_entity(&mut commands, &map, &mut pending, entity_id);
            }
            RowEvent::StatModifierRemoved(row) => {
                pending.stat_modifier.remove(&row.id);
                replay_entity(&mut commands, &map, &mut pending, row.entity_id);
            }
            RowEvent::PeriodicEffect(row) => {
                let entity_id = row.entity_id;
                pending.periodic_effect.insert(row.id, row);
                replay_entity(&mut commands, &map, &mut pending, entity_id);
            }
            RowEvent::PeriodicEffectRemoved(row) => {
                pending.periodic_effect.remove(&row.id);
                replay_entity(&mut commands, &map, &mut pending, row.entity_id);
            }
            RowEvent::Cooldown(row) => {
                cooldowns.write(SpellCooldownState {
                    entity_id: row.entity_id,
                    ability_id: row.ability_id,
                    remaining_seconds: (row.duration_seconds - row.elapsed_seconds).max(0.0),
                    duration_seconds: row.duration_seconds,
                });
            }
            RowEvent::CooldownRemoved(row) => {
                // The row is gone, so the ability is ready: a zero remainder is
                // how the HUD is told to clear the overlay.
                cooldowns.write(SpellCooldownState {
                    entity_id: row.entity_id,
                    ability_id: row.ability_id,
                    remaining_seconds: 0.0,
                    duration_seconds: row.duration_seconds,
                });
            }
            RowEvent::Projectile(row) => {
                apply_projectile(&mut commands, &mut map, &row);
            }
            RowEvent::ProjectileRemoved(id) => {
                if let Some(entity) = map.projectiles.remove(&id) {
                    commands.entity(entity).despawn();
                }
            }
            RowEvent::PlayerMessage(row) => {
                // `target` of `None` is a broadcast. A targeted message only
                // reaches this client if the server addressed it here, but the
                // table is public, so the check is made rather than assumed.
                if row.target.is_none() || row.target == local_identity {
                    notices.write(ServerNotice::info(row.text));
                }
            }
            RowEvent::ReducerRejected(message) => {
                notices.write(ServerNotice::error(message));
            }
            RowEvent::JoinRejected(message) => {
                failure.0 = Some(message);
                screen.0 = Screen::MainMenu;
            }
        }
    }
}

fn replay_entity(
    commands: &mut Commands,
    map: &StdbEntityMap,
    pending: &mut PendingRows,
    entity_id: u64,
) {
    let Some(entity) = map.get(entity_id) else {
        return;
    };

    if let Some(row) = pending.stats.get(&entity_id) {
        apply_stats(commands, entity, row);
    }
    if let Some(row) = pending.boss_state.get(&entity_id) {
        apply_boss_state(commands, entity, row);
    }
    apply_effects(commands, entity, entity_id, pending);
}

/// Rewrites `CrowdControlState` and `ActiveStatModifiers` — but only when the
/// rows behind them actually changed.
///
/// Both components are derived from a set of rows rather than from a single
/// one, so there is nothing to update in place: they are rebuilt whole. The
/// guard is what keeps that from being a per-tick write on every entity in the
/// world, since this runs from `replay_entity` and `replay_entity` runs on
/// every `game_entity` update.
fn apply_effects(
    commands: &mut Commands,
    entity: Entity,
    entity_id: u64,
    pending: &mut PendingRows,
) {
    let crowd_control = crowd_control_state_for(entity_id, pending);
    let modifier_signature = modifier_signature_for(entity_id, pending);
    let next = AppliedEffects {
        crowd_control,
        modifier_signature,
    };

    if pending.applied.get(&entity_id) == Some(&next) {
        return;
    }

    commands
        .entity(entity)
        .insert((next.crowd_control.clone(), stat_modifiers_for(entity_id, pending)));
    pending.applied.insert(entity_id, next);
}

fn replay_identity(
    commands: &mut Commands,
    map: &StdbEntityMap,
    pending: &PendingRows,
    identity: Identity,
    local_identity: Option<Identity>,
) {
    let Some(entity) = entity_for(map, identity) else {
        return;
    };

    if let Some(row) = pending.inventory.get(&identity) {
        commands.entity(entity).insert(inventory_from(&row.slots));
    }
    if let Some(row) = pending.equipment.get(&identity) {
        commands.entity(entity).insert(equipment_from(&row.slots));
    }
    if let Some(row) = pending.hotbar.get(&identity) {
        commands.entity(entity).insert(hotbar_from(row));
    }
    if local_identity == Some(identity) {
        if let Some(row) = pending.known_glyphs.get(&identity) {
            commands.entity(entity).insert(known_glyphs_from(row));
        }
    }
}

fn entity_for(map: &StdbEntityMap, identity: Identity) -> Option<Entity> {
    map.entity_of_identity
        .get(&identity)
        .and_then(|id| map.get(*id))
}

fn apply_stats(commands: &mut Commands, entity: Entity, row: &EntityStats) {
    let stats = &row.stats;
    commands.entity(entity).insert((
        VitalStats {
            current_health: stats.current_health,
            max_health: stats.max_health,
            max_mana: stats.max_mana,
            mana_regeneration: stats.mana_regeneration,
        },
        CombatStats {
            armor: stats.armor,
            attack_power: stats.attack_power,
        },
        MovementStats {
            speed: stats.movement_speed,
        },
    ));
}

fn hotbar_from(row: &Hotbar) -> SpellHotbar {
    SpellHotbar {
        q_spell: row.slots.q.clone().map(SpellId::new),
        w_spell: row.slots.w.clone().map(SpellId::new),
        e_spell: row.slots.e.clone().map(SpellId::new),
    }
}

fn known_glyphs_from(row: &KnownGlyphsTable) -> KnownGlyphs {
    KnownGlyphs {
        essences: row.essences.iter().cloned().map(EssenceId::new).collect(),
        modifiers: row.modifiers.iter().cloned().map(ModifierId::new).collect(),
        ancient_words: row
            .ancient_words
            .iter()
            .cloned()
            .map(AncientWordId::new)
            .collect(),
    }
}

fn cast_progress_from(row: &CastState) -> SpellCastProgress {
    SpellCastProgress {
        caster_network_id: row.entity_id,
        spell_id: row.spell_id.clone(),
        kind: match row.kind {
            CastKindRow::Channeling => 1,
            CastKindRow::Instant | CastKindRow::CastTime => 0,
        },
        elapsed_seconds: row.elapsed_seconds,
        required_seconds: row.required_seconds,
    }
}

fn apply_boss_state(commands: &mut Commands, entity: Entity, row: &BossState) {
    commands.entity(entity).insert((
        BossArena {
            center: to_vec3(&row.arena_center),
            radius: row.arena_radius,
            is_engaged: row.is_engaged,
        },
        boss_phase(row.phase),
    ));
}

fn boss_phase(phase: BossPhaseRow) -> BossPhase {
    match phase {
        BossPhaseRow::Idle => BossPhase::Dormant,
        BossPhaseRow::PhaseOne => BossPhase::Ground,
        BossPhaseRow::PhaseTwo => BossPhase::Aerial,
        BossPhaseRow::Enraged => BossPhase::Berserk,
    }
}

/// Collects one entity's crowd control into the component the UI queries.
///
/// `Root`, `Silence` and `Slow` are dropped rather than approximated: the
/// domain's `CrowdControlKind` knows only `Stun`, and inventing a mapping here
/// would put a bar on screen that no gating rule agrees with. Nothing emits
/// them today, so the branch is a guard against a future module change landing
/// silently, not a live gap.
fn crowd_control_state_for(entity_id: u64, pending: &PendingRows) -> CrowdControlState {
    let effects = pending
        .crowd_control
        .values()
        .filter(|row| row.entity_id == entity_id)
        .filter_map(|row| {
            let kind = match row.kind {
                CrowdControlKindRow::Stun => CrowdControlKind::Stun,
                other => {
                    debug!(
                        "omitting non-Stun CrowdControl row: entity={entity_id}, kind={other:?}"
                    );
                    return None;
                }
            };
            Some(ActiveCrowdControl {
                kind,
                remaining_seconds: row.remaining_seconds,
                total_seconds: row.total_seconds,
            })
        })
        .collect();
    CrowdControlState { effects }
}

/// The rows behind one entity's `ActiveStatModifiers`, as a comparable key.
///
/// Durations are compared by their bit pattern because `f32` has no `Eq`, and
/// an exact-equality test is the right one here: the value either came through
/// unchanged from the last row event or it did not.
fn modifier_signature_for(entity_id: u64, pending: &PendingRows) -> Vec<(u64, u32)> {
    let mut signature: Vec<(u64, u32)> = pending
        .stat_modifier
        .values()
        .filter(|row| row.entity_id == entity_id)
        .map(|row| (row.id, row.remaining_seconds.unwrap_or(f32::INFINITY).to_bits()))
        .chain(
            pending
                .periodic_effect
                .values()
                .filter(|row| row.entity_id == entity_id)
                // Periodic ids share the key space with modifier ids here, so
                // they are offset to keep the two apart.
                .map(|row| (row.id ^ (1 << 63), row.remaining_seconds.to_bits())),
        )
        .collect();
    signature.sort_unstable();
    signature
}

/// Rebuilds one entity's buff and debuff list from the two tables that feed it.
///
/// `stat_modifier` and `periodic_effect` are separate rows on the server — a
/// modifier changes what a stat *is*, a periodic effect changes health on a
/// schedule — but the domain component the UI reads holds both, as variants of
/// `ModifierEffectInstance`. One row becomes one instance with one effect;
/// nothing here needs to merge them, because the server already refreshes
/// rather than stacks.
fn stat_modifiers_for(entity_id: u64, pending: &PendingRows) -> ActiveStatModifiers {
    let stat_effects = pending
        .stat_modifier
        .values()
        .filter(|row| row.entity_id == entity_id)
        .map(|row| StatModifierInstance {
            id: StatModifierId(row.id),
            source: row.source.map(EntityId::new),
            effects: vec![ModifierEffectInstance::Stat {
                field: stat_field_from(&row.field),
                operation: if row.is_multiplicative {
                    ModifierOp::Multiply
                } else {
                    ModifierOp::Add
                },
                value: row.amount,
            }],
            remaining_seconds: row.remaining_seconds,
            kind: match row.kind {
                ModifierKindRow::Buff => ModifierKind::Buff,
                ModifierKindRow::Debuff => ModifierKind::Debuff,
            },
        });

    let periodic_effects = pending
        .periodic_effect
        .values()
        .filter(|row| row.entity_id == entity_id)
        .map(|row| StatModifierInstance {
            // Offset to keep the two id spaces from colliding, matching
            // `modifier_signature_for`.
            id: StatModifierId(row.id ^ (1 << 63)),
            source: row.source.map(EntityId::new),
            // The table stores one signed number; the domain wants the sign as
            // the variant and the magnitude as the value.
            effects: vec![if row.amount_per_tick >= 0.0 {
                ModifierEffectInstance::HealOverTime {
                    amount_per_tick: row.amount_per_tick,
                    tick_interval: row.tick_interval_seconds,
                    time_since_last_tick: row.since_last_tick,
                }
            } else {
                ModifierEffectInstance::DamageOverTime {
                    amount_per_tick: -row.amount_per_tick,
                    tick_interval: row.tick_interval_seconds,
                    time_since_last_tick: row.since_last_tick,
                }
            }],
            remaining_seconds: Some(row.remaining_seconds),
            kind: if row.amount_per_tick >= 0.0 {
                ModifierKind::Buff
            } else {
                ModifierKind::Debuff
            },
        });

    ActiveStatModifiers {
        modifiers: stat_effects.chain(periodic_effects).collect(),
    }
}

/// Parses the module's `StatField` debug name back into the enum.
///
/// The module stores the name rather than an ordinal so `spacetime sql` stays
/// readable, which makes this the inverse of its `stat_field_name`. An
/// unrecognised name means the two have drifted; `Speed` is the least harmful
/// landing spot and the warning says what happened.
fn stat_field_from(name: &str) -> StatField {
    match name {
        "Speed" => StatField::Speed,
        "Armor" => StatField::Armor,
        "AttackPower" => StatField::AttackPower,
        "MaxHealth" => StatField::MaxHealth,
        "ManaRegeneration" => StatField::ManaRegeneration,
        other => {
            warn!("unknown stat field {other:?} from the module; treating it as Speed");
            StatField::Speed
        }
    }
}

/// Spawns or updates the Bevy entity mirroring one `projectile` row.
///
/// Projectiles are not `game_entity` rows, so nothing else mirrors them — which
/// is why every spell that fires one had a visual effect at the muzzle and
/// nothing in between. `ProjectileVisual` is what tells the renderer to draw the
/// small emissive cube rather than a character model.
fn apply_projectile(commands: &mut Commands, map: &mut StdbEntityMap, row: &Projectile) {
    let position = to_vec3(&row.position);

    match map.projectiles.get(&row.id).copied() {
        Some(entity) => {
            commands.entity(entity).insert(Position(position));
        }
        None => {
            let entity = commands
                .spawn((
                    Position(position),
                    ProjectileVisual {
                        spell_id: row.spell_id.clone(),
                    },
                    // The renderer needs a colour whether or not it uses this
                    // one: the projectile material is shared when the asset
                    // cache is warm, and this is the fallback tint.
                    EntityColor(Color::srgb(0.2, 0.7, 1.0)),
                ))
                .id();
            map.projectiles.insert(row.id, entity);
        }
    }
}

/// Spawns or updates the Bevy entity mirroring one `game_entity` row.
fn apply_entity(
    commands: &mut Commands,
    map: &mut StdbEntityMap,
    row: &EntityRow,
    local_identity: Option<Identity>,
) {
    let authoritative = StdbAuthoritative {
        position: to_vec3(&row.position),
        move_target: row.move_target.as_ref().map(to_vec3),
        speed: row.speed,
    };

    let existing = map.by_entity_id.get(&row.entity_id).copied();
    let entity = match existing {
        Some(entity) => entity,
        None => {
            let entity = commands
                .spawn((
                    GameEntity,
                    NetworkEntityId(row.entity_id),
                    // Seeded from the authoritative position so a character does
                    // not visibly glide in from the origin on its first frame.
                    Position(authoritative.position),
                    PlayerName(row.display_name.clone()),
                ))
                .id();
            map.by_entity_id.insert(row.entity_id, entity);
            if row.owner.is_some() {
                // Recorded here as well as from the `player` table, because the
                // two rows can arrive in either order.
                if let Some(identity) = row.owner {
                    map.entity_of_identity.insert(identity, row.entity_id);
                }
            }
            debug!(
                "mirrored {:?} {} as entity {} at {}",
                row.kind, row.display_name, row.entity_id, authoritative.position
            );
            entity
        }
    };

    let mut cmd = commands.entity(entity);
    cmd.insert((
        authoritative,
        entity_color(&row.color),
        LookDirection(to_vec3(&row.look)),
        entity_kind(row.kind),
        entity_state(row.state),
    ));
    if matches!(row.kind, EntityKindRow::Boss) {
        cmd.insert(Boss);
    }
    if local_identity.is_some() && row.owner == local_identity {
        cmd.insert(LocalPlayer);
    }
}

fn entity_color(color: &ColorRow) -> EntityColor {
    EntityColor(Color::srgba(
        color.red,
        color.green,
        color.blue,
        color.alpha,
    ))
}

/// Maps the module's entity kinds onto the presentation's.
///
/// The module distinguishes `Enemy`/`Boss`/`Dummy` because the simulation
/// treats them differently; the client only cares whether something is hostile,
/// which is why the two enums are not the same shape.
fn entity_kind(kind: EntityKindRow) -> EntityKind {
    match kind {
        EntityKindRow::Player => EntityKind::Player,
        EntityKindRow::Npc => EntityKind::Friendly,
        EntityKindRow::Dummy => EntityKind::Neutral,
        EntityKindRow::Enemy | EntityKindRow::Boss => EntityKind::Hostile,
    }
}

fn entity_state(state: EntityStateRow) -> EntityState {
    match state {
        EntityStateRow::Idle => EntityState::Idle,
        EntityStateRow::Moving => EntityState::Moving,
        EntityStateRow::Dead => EntityState::Dead,
    }
}

fn inventory_from(slots: &[Option<ItemInstanceRow>]) -> Inventory {
    let mut inventory = Inventory::default();
    for (slot, row) in inventory.slots.iter_mut().zip(slots) {
        *slot = row.as_ref().map(item_instance_from);
    }
    inventory
}

fn equipment_from(slots: &[Option<ItemInstanceRow>]) -> Equipment {
    use bevymmo_shared::items::EquipSlot;
    // Same order the module writes them in; see `rows::EQUIP_SLOTS`.
    const ORDER: [EquipSlot; 10] = [
        EquipSlot::Bag,
        EquipSlot::Helmet,
        EquipSlot::Cape,
        EquipSlot::Weapon,
        EquipSlot::Armor,
        EquipSlot::Offhand,
        EquipSlot::Potion,
        EquipSlot::Shoes,
        EquipSlot::Food,
        EquipSlot::Mount,
    ];
    let mut equipment = Equipment::default();
    for (slot, row) in ORDER.iter().zip(slots) {
        *equipment.get_mut(*slot) = row.as_ref().map(item_instance_from);
    }
    equipment
}

fn item_instance_from(row: &ItemInstanceRow) -> bevymmo_shared::items::instance::ItemInstance {
    use bevymmo_shared::abilities::inscription::{Inscription, WeaponInscriptions};
    use bevymmo_shared::abilities::weapon_abilities::AbilitySelection;
    use bevymmo_shared::abilities::{AbilityId, AncientWordId, EssenceId, ModifierId};
    use bevymmo_shared::items::instance::{ItemInstance, ItemInstanceId};
    use bevymmo_shared::items::registry::ItemId;

    let inscription = |i: &super::module_bindings::InscriptionRow| Inscription {
        essence: i.essence.clone().map(EssenceId::new),
        modifiers: i.modifiers.iter().cloned().map(ModifierId::new).collect(),
        ancient_word: i.ancient_word.clone().map(AncientWordId::new),
    };

    ItemInstance {
        instance_id: ItemInstanceId(row.instance_id),
        item_id: ItemId::new(row.item_id.clone()),
        inscriptions: row.inscriptions.as_ref().map(|w| WeaponInscriptions {
            primary: inscription(&w.primary),
            secondary: inscription(&w.secondary),
            ultimate: inscription(&w.ultimate),
        }),
        ability_selection: AbilitySelection {
            primary: row.ability_selection.primary.clone().map(AbilityId::new),
            secondary: row.ability_selection.secondary.clone().map(AbilityId::new),
        },
    }
}

/// Turns the main menu's "connect as <name>" into a `join` call and handles logout.
///
/// Reuses `ConnectionRequest`, the same resource the lightyear path consumed, so
/// the menu does not need to know which transport is mounted.
fn join_on_request(
    conn: Res<StdbConnection>,
    config: Res<StdbConnectionConfig>,
    mut request: ResMut<ConnectionRequest>,
    mut screen: ResMut<GameScreen>,
    mut failure: ResMut<ConnectionFailure>,
    mut map: ResMut<StdbEntityMap>,
    mut pending: ResMut<PendingRows>,
    mut commands: Commands,
) {
    let Some(intent) = request.0.take() else {
        return;
    };

    match intent {
        ConnectionIntent::Connect { player_name } => {
            match conn
                .reducers()
                .join_then(player_name.clone(), conn.report_join_rejection())
            {
                Ok(()) => {
                    info!("joining SpacetimeDB as {player_name}");
                    // Optimistic: the reducer is authoritative and may still reject the
                    // name, in which case `player` never gains a row and the character
                    // never appears.
                    screen.0 = Screen::InGame;
                }
                Err(err) => {
                    error!("join failed: {err}");
                    failure.0 = Some(format!("Impossibile connettersi: {err}"));
                }
            }
        }
        ConnectionIntent::Logout => {
            if let Err(err) = conn.conn.disconnect() {
                warn!("could not disconnect during logout: {err}");
            }
            clear_cached_token();
            clear_replicated_state(&mut commands, &mut map, &mut pending);

            match connect(&config.uri, &config.module) {
                Ok(connection) => {
                    commands.insert_resource(connection);
                    failure.0 = None;
                    screen.0 = Screen::MainMenu;
                }
                Err(err) => {
                    error!("SpacetimeDB reconnection after logout failed: {err}");
                    failure.0 = Some(format!("Impossibile riconnettersi: {err}"));
                }
            }
        }
        ConnectionIntent::Disconnect => {}
    }
}

/// Drops all rows and Bevy entities mirrored from the previous connection.
///
/// A new identity receives its own subscription snapshot. Retaining this state
/// would leave the old character marked as [`LocalPlayer`], making both the
/// camera and input target multiple entities after logging out.
fn clear_replicated_state(
    commands: &mut Commands,
    map: &mut StdbEntityMap,
    pending: &mut PendingRows,
) {
    for entity in map
        .by_entity_id
        .drain()
        .map(|(_, entity)| entity)
        .chain(map.projectiles.drain().map(|(_, entity)| entity))
    {
        commands.entity(entity).despawn();
    }
    map.entity_of_identity.clear();
    *pending = PendingRows::default();
}

/// Tells the server the client is still here.
fn send_heartbeat(conn: Res<StdbConnection>, time: Res<Time>, mut elapsed: Local<f32>) {
    *elapsed += time.delta_secs();
    if *elapsed < HEARTBEAT_INTERVAL {
        return;
    }
    *elapsed = 0.0;
    // Fails harmlessly before `join`: there is no character to mark present yet.
    let _ = conn.reducers().heartbeat();
}

/// Held right mouse button sets the destination, as it always has.
fn send_move_commands(
    conn: Res<StdbConnection>,
    time: Res<Time>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    surface_query: Res<ClientSurfaceQuery>,
    mut cooldown: Local<f32>,
) {
    let Some(mouse) = mouse else {
        return;
    };
    if !mouse.pressed(MouseButton::Right) {
        *cooldown = 0.0;
        return;
    }

    let just_pressed = mouse.just_pressed(MouseButton::Right);
    *cooldown -= time.delta_secs();
    // The first press goes out immediately; while held, the destination is
    // resent at a fixed rate so the character follows the pointer without one
    // reducer call per frame.
    if !just_pressed && *cooldown > 0.0 {
        return;
    }
    *cooldown = MOVE_COMMAND_INTERVAL;

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else {
        return;
    };
    // When no terrain mesh is available, fall back to a horizontal plane at Y=0.
    // This is safe: the server ignores the client-sent Y and resolves X/Z
    // authoritatively against its own collision data.
    //
    // A ray parallel to that plane never meets it, and dividing by its zero Y
    // would send an infinite coordinate the module can only reject — so that
    // case is simply not a click.
    let Some(point) = surface_query
        .0
        .as_ref()
        .and_then(|sq| resolve_ray_to_ground(ray.origin, *ray.direction, sq, 100.0, 0.5))
        .or_else(|| {
            let t = -ray.origin.y / ray.direction.y;
            t.is_finite().then(|| ray.origin + *ray.direction * t)
        })
    else {
        return;
    };

    if let Err(err) = conn.reducers().move_to(point.x, point.y, point.z) {
        error!("move_to failed: {err}");
    }
}

/// Advances every entity towards its destination, then eases the result back
/// towards what the server last said.
///
/// Runs for remote entities as much as the local one: at ~18 Hz, interpolating
/// other characters is what makes them walk instead of teleport between updates.
fn predict_and_reconcile(time: Res<Time>, mut query: Query<(&mut Position, &StdbAuthoritative)>) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for (mut position, authoritative) in &mut query {
        if let Some(target) = authoritative.move_target {
            position.0 = match movement::step_towards(position.0, target, authoritative.speed, dt) {
                Step::Moving(p) | Step::Arrived(p) => p,
            };
        }

        let error = authoritative.position - position.0;
        let drift = error.length();
        if drift > SNAP_DISTANCE {
            position.0 = authoritative.position;
        } else if drift > 0.0 {
            // Exponential approach, framerate-independent: the fraction closed
            // per second is constant regardless of how the frames fall.
            position.0 += error * (1.0 - (-RECONCILE_RATE * dt).exp());
        }
    }
}

fn to_vec3(v: &Vec3Row) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdb::module_bindings::CastSourceRow;

    #[test]
    fn color_row_becomes_entity_color() {
        let row = ColorRow {
            red: 0.2,
            green: 0.4,
            blue: 0.6,
            alpha: 0.8,
        };

        assert_eq!(
            entity_color(&row),
            EntityColor(Color::srgba(0.2, 0.4, 0.6, 0.8))
        );
    }

    #[test]
    fn known_glyph_row_becomes_domain_component() {
        let row = KnownGlyphsTable {
            identity: Identity::default(),
            essences: vec!["fire".to_string()],
            modifiers: vec!["amplify".to_string()],
            ancient_words: vec!["eternity".to_string()],
        };

        let glyphs = known_glyphs_from(&row);

        assert!(glyphs.essences.contains(&EssenceId::new("fire")));
        assert!(glyphs.modifiers.contains(&ModifierId::new("amplify")));
        assert!(glyphs
            .ancient_words
            .contains(&AncientWordId::new("eternity")));
    }

    #[test]
    fn cast_state_becomes_legacy_cast_progress() {
        let row = CastState {
            entity_id: 42,
            spell_id: "ray_of_light".to_string(),
            kind: CastKindRow::Channeling,
            source: CastSourceRow::Spell, // Legacy spell
            elapsed_seconds: 1.5,
            required_seconds: 3.0,
            start_position: Vec3Row {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            target_position: None,
            target_entity: None,
            channel_tick_accumulator: 0.0,
            tick_interval_seconds: 0.25,
            channel_movement_interrupts: true, // Standard interrupt-on-move
        };

        assert_eq!(
            cast_progress_from(&row),
            SpellCastProgress {
                caster_network_id: 42,
                spell_id: "ray_of_light".to_string(),
                kind: 1,
                elapsed_seconds: 1.5,
                required_seconds: 3.0,
            }
        );
    }

    #[test]
    fn boss_phases_match_existing_presentation_contract() {
        assert_eq!(boss_phase(BossPhaseRow::Idle), BossPhase::Dormant);
        assert_eq!(boss_phase(BossPhaseRow::PhaseOne), BossPhase::Ground);
        assert_eq!(boss_phase(BossPhaseRow::PhaseTwo), BossPhase::Aerial);
        assert_eq!(boss_phase(BossPhaseRow::Enraged), BossPhase::Berserk);
    }

    fn crowd_control_row(
        id: u64,
        entity_id: u64,
        kind: CrowdControlKindRow,
        remaining_seconds: f32,
        total_seconds: f32,
    ) -> CrowdControl {
        CrowdControl {
            id,
            entity_id,
            source: None,
            kind,
            remaining_seconds,
            total_seconds,
        }
    }

    #[test]
    fn crowd_control_projects_only_representable_stuns() {
        let mut pending = PendingRows::default();
        pending.crowd_control.insert(
            1,
            crowd_control_row(1, 7, CrowdControlKindRow::Stun, 1.5, 2.0),
        );
        pending.crowd_control.insert(
            2,
            crowd_control_row(2, 7, CrowdControlKindRow::Root, 1.0, 1.0),
        );

        let state = crowd_control_state_for(7, &pending);

        assert_eq!(state.effects.len(), 1);
        assert_eq!(state.effects[0].kind, CrowdControlKind::Stun);
        assert_eq!(state.effects[0].remaining_seconds, 1.5);
        // Read from the row now, not guessed from the largest remainder ever
        // seen: a bar that joins a stun in progress starts part-full, as it
        // should, instead of snapping to full on the first frame.
        assert_eq!(state.effects[0].total_seconds, 2.0);
    }

    #[test]
    fn unchanged_effects_are_not_rewritten() {
        let mut pending = PendingRows::default();
        pending
            .crowd_control
            .insert(1, crowd_control_row(1, 7, CrowdControlKindRow::Stun, 1.5, 2.0));

        let first = AppliedEffects {
            crowd_control: crowd_control_state_for(7, &pending),
            modifier_signature: modifier_signature_for(7, &pending),
        };
        pending.applied.insert(7, first);

        let unchanged = AppliedEffects {
            crowd_control: crowd_control_state_for(7, &pending),
            modifier_signature: modifier_signature_for(7, &pending),
        };
        assert_eq!(pending.applied.get(&7), Some(&unchanged));

        pending
            .crowd_control
            .insert(1, crowd_control_row(1, 7, CrowdControlKindRow::Stun, 1.0, 2.0));
        let ticked = AppliedEffects {
            crowd_control: crowd_control_state_for(7, &pending),
            modifier_signature: modifier_signature_for(7, &pending),
        };
        assert_ne!(pending.applied.get(&7), Some(&ticked));
    }

    #[test]
    fn periodic_effects_become_over_time_modifiers() {
        let mut pending = PendingRows::default();
        pending.periodic_effect.insert(
            3,
            PeriodicEffect {
                id: 3,
                entity_id: 7,
                source: Some(9),
                amount_per_tick: -4.0,
                tick_interval_seconds: 0.5,
                since_last_tick: 0.1,
                remaining_seconds: 6.0,
            },
        );

        let modifiers = stat_modifiers_for(7, &pending);

        assert_eq!(modifiers.modifiers.len(), 1);
        let instance = &modifiers.modifiers[0];
        assert_eq!(instance.source, Some(EntityId::new(9)));
        assert_eq!(instance.kind, ModifierKind::Debuff);
        // The table stores one signed number; the domain wants the sign as the
        // variant and the magnitude as the value.
        assert_eq!(
            instance.effects,
            vec![ModifierEffectInstance::DamageOverTime {
                amount_per_tick: 4.0,
                tick_interval: 0.5,
                time_since_last_tick: 0.1,
            }]
        );
    }
}
