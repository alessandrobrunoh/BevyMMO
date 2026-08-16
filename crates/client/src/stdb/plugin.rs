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
use bevymmo_shared::stats::components::{CombatStats, MovementStats, VitalStats};
use bevymmo_shared::world_components::{EntityColor, LookDirection, NetworkEntityId, Position};
use crossbeam_channel::{unbounded, Receiver, Sender};
use spacetimedb_sdk::{credentials, DbContext, EventTable, Identity, Table, TableWithPrimaryKey};
use std::collections::HashMap;

use super::module_bindings::boss_state_table::BossStateTableAccess;
use super::module_bindings::cast_ended_table::CastEndedTableAccess;
use super::module_bindings::cast_state_table::CastStateTableAccess;
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
use super::module_bindings::player_table::PlayerTableAccess;
use super::module_bindings::spell_visual_effect_table::SpellVisualEffectTableAccess;
use super::module_bindings::{
    BossPhaseRow, BossState, CastEndedEvent, CastKindRow, CastState, ColorRow, CrowdControl,
    CrowdControlKindRow, DbConnection, EntityKindRow, EntityStateRow, EntityStats, EquipmentTable,
    GameEntity as EntityRow, Hotbar, InventoryTable, ItemInstanceRow, KnownGlyphsTable, Player,
    RemoteReducers, SpellVisualEffectEvent, Vec3Row,
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
}

/// Latest rows retained until the dependent Bevy entity exists. Initial
/// subscription rows have no delivery order guarantee.
#[derive(Resource, Default)]
/// Caches the latest received rows per entity so that `replay_entity` can
/// restore full component state on (re)connect or after a reconcile gap.
struct PendingRows {
    stats: HashMap<u64, EntityStats>,
    inventory: HashMap<Identity, InventoryTable>,
    equipment: HashMap<Identity, EquipmentTable>,
    hotbar: HashMap<Identity, Hotbar>,
    known_glyphs: HashMap<Identity, KnownGlyphsTable>,
    boss_state: HashMap<u64, BossState>,
    crowd_control: HashMap<u64, PendingCrowdControl>,
}

struct PendingCrowdControl {
    row: CrowdControl,
    total_seconds: f32,
}

/// Owns the connection. Call reducers through [`StdbConnection::reducers`].
#[derive(Resource)]
pub struct StdbConnection {
    conn: DbConnection,
    events: Receiver<RowEvent>,
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
}

/// Maps server entity ids to the Bevy entities mirroring them.
#[derive(Resource, Default)]
pub struct StdbEntityMap {
    by_entity_id: HashMap<u64, Entity>,
    /// Which server entity belongs to which account, so the per-character
    /// tables (inventory, equipment, hotbar) can find the entity to attach to.
    entity_of_identity: HashMap<Identity, u64>,
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

fn connect(uri: &str, module: &str) -> Result<StdbConnection, Box<dyn std::error::Error>> {
    let (tx, events) = unbounded();

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
            "SELECT * FROM cast_ended",
            "SELECT * FROM spell_visual_effect",
        ]);

    Ok(StdbConnection { conn, events })
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

    let cast_ended = tx.clone();
    conn.db().cast_ended().on_insert(move |_ctx, row| {
        let _ = cast_ended.send(RowEvent::CastEnded(row.clone()));
    });
    let spell_visual_effect = tx.clone();
    conn.db().spell_visual_effect().on_insert(move |_ctx, row| {
        let _ = spell_visual_effect.send(RowEvent::SpellVisualEffect(row.clone()));
    });

    let crowd_control_removed = tx.clone();
    conn.db().crowd_control().on_delete(move |_ctx, row| {
        let _ = crowd_control_removed.send(RowEvent::CrowdControlRemoved(row.clone()));
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
) {
    let local_identity = conn.identity();

    while let Ok(event) = conn.events.try_recv() {
        match event {
            RowEvent::Entity(row) => {
                apply_entity(&mut commands, &mut map, &row, local_identity);
                replay_entity(&mut commands, &map, &pending, row.entity_id);
                if let Some(identity) = row.owner {
                    replay_identity(&mut commands, &map, &pending, identity, local_identity);
                }
            }
            RowEvent::EntityRemoved(entity_id) => {
                if let Some(entity) = map.by_entity_id.remove(&entity_id) {
                    commands.entity(entity).despawn();
                }
                map.entity_of_identity.retain(|_, id| *id != entity_id);
                pending.stats.remove(&entity_id);
                pending.boss_state.remove(&entity_id);
                pending
                    .crowd_control
                    .retain(|_, cc| cc.row.entity_id != entity_id);
            }
            RowEvent::Stats(row) => {
                pending.stats.insert(row.entity_id, row.clone());
                replay_entity(&mut commands, &map, &pending, row.entity_id);
            }
            RowEvent::Player(row) => {
                map.entity_of_identity.insert(row.identity, row.entity_id);
                replay_identity(&mut commands, &map, &pending, row.identity, local_identity);
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
                pending.boss_state.insert(row.entity_id, row.clone());
                replay_entity(&mut commands, &map, &pending, row.entity_id);
            }
            RowEvent::CrowdControl(row) => {
                let total_seconds = pending
                    .crowd_control
                    .get(&row.id)
                    .map(|existing| {
                        if row.remaining_seconds > existing.row.remaining_seconds {
                            row.remaining_seconds
                        } else {
                            existing.total_seconds
                        }
                    })
                    .unwrap_or(row.remaining_seconds);
                let entity_id = row.entity_id;
                pending
                    .crowd_control
                    .insert(row.id, PendingCrowdControl { row, total_seconds });
                replay_entity(&mut commands, &map, &pending, entity_id);
            }
            RowEvent::CrowdControlRemoved(row) => {
                pending.crowd_control.remove(&row.id);
                replay_entity(&mut commands, &map, &pending, row.entity_id);
            }
        }
    }
}

fn replay_entity(
    commands: &mut Commands,
    map: &StdbEntityMap,
    pending: &PendingRows,
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
    apply_crowd_control(commands, entity, entity_id, pending);
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

fn apply_crowd_control(
    commands: &mut Commands,
    entity: Entity,
    entity_id: u64,
    pending: &PendingRows,
) {
    commands
        .entity(entity)
        .insert(crowd_control_state_for(entity_id, pending));
}

fn crowd_control_state_for(entity_id: u64, pending: &PendingRows) -> CrowdControlState {
    let effects = pending
        .crowd_control
        .values()
        .filter(|cc| cc.row.entity_id == entity_id)
        .filter_map(|cc| {
            let kind = match cc.row.kind {
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
                remaining_seconds: cc.row.remaining_seconds,
                total_seconds: cc.total_seconds,
            })
        })
        .collect();
    CrowdControlState { effects }
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

/// Turns the main menu's "connect as <name>" into a `join` call.
///
/// Reuses `ConnectionRequest`, the same resource the lightyear path consumed, so
/// the menu does not need to know which transport is mounted.
fn join_on_request(
    conn: Res<StdbConnection>,
    mut request: ResMut<ConnectionRequest>,
    mut screen: ResMut<GameScreen>,
    mut failure: ResMut<ConnectionFailure>,
) {
    let Some(intent) = request.0.take() else {
        return;
    };
    let ConnectionIntent::Connect { player_name } = intent else {
        return;
    };

    match conn.reducers().join(player_name.clone()) {
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
    let point = surface_query
        .0
        .as_ref()
        .and_then(|sq| resolve_ray_to_ground(ray.origin, *ray.direction, sq, 100.0, 0.5))
        .unwrap_or_else(|| {
            let t = -ray.origin.y / ray.direction.y;
            ray.origin + *ray.direction * t
        });

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

    #[test]
    fn crowd_control_projects_only_representable_stuns() {
        let mut pending = PendingRows::default();
        pending.crowd_control.insert(
            1,
            PendingCrowdControl {
                row: CrowdControl {
                    id: 1,
                    entity_id: 7,
                    kind: CrowdControlKindRow::Stun,
                    remaining_seconds: 1.5,
                },
                total_seconds: 2.0,
            },
        );
        pending.crowd_control.insert(
            2,
            PendingCrowdControl {
                row: CrowdControl {
                    id: 2,
                    entity_id: 7,
                    kind: CrowdControlKindRow::Root,
                    remaining_seconds: 1.0,
                },
                total_seconds: 1.0,
            },
        );

        let state = crowd_control_state_for(7, &pending);

        assert_eq!(state.effects.len(), 1);
        assert_eq!(state.effects[0].kind, CrowdControlKind::Stun);
        assert_eq!(state.effects[0].remaining_seconds, 1.5);
        assert_eq!(state.effects[0].total_seconds, 2.0);
    }
}
