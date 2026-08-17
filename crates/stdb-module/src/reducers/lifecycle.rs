//! Module startup, connections, and character creation.

use spacetimedb::{reducer, ReducerContext, ScheduleAt, Table};
use std::time::Duration;

use crate::rows::{equipment_to_rows, inventory_to_rows, HotbarRow, StatsRow, Vec3Row};
use crate::tables::{
    active_status, aoe_region, boss_state, cast_state, cooldown, crowd_control, entity_stats, equipment,
    game_entity, grid_cell, hotbar, inventory, known_ancient_language, known_glyphs, periodic_effect,
    player, player_stats,
    projectile, resonance, stat_modifier, threat, tick_schedule, tick_stats, ColorRow, EntityKindRow,
    EntityStateRow, EquipmentTable, GameEntity, Hotbar, InventoryTable, KnownAncientLanguageTable,
    KnownGlyphsTable, Player,
    PlayerStats, TickSchedule,
};
use crate::{normalize_name, world, DEFAULT_SPEED_PER_SECOND, TICK_INTERVAL_MS};

/// Runs once, when the module is first published to an empty database.
#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    clear_runtime_state(ctx);

    ctx.db.tick_schedule().insert(TickSchedule {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Interval(Duration::from_millis(TICK_INTERVAL_MS).into()),
    });

    world::seed(ctx);

    log::info!("module initialised; tick every {TICK_INTERVAL_MS} ms");
}

/// Wipes everything that models a live session.
///
/// Necessary because SpacetimeDB persists every table, including the ones that
/// only make sense while the server is up. Without this a republish inherits
/// mid-flight projectiles, half-finished casts and stale threat tables.
///
/// Player *characters* are deliberately untouched — those are the persistent
/// half, and losing them on every publish would be the opposite of the point.
fn clear_runtime_state(ctx: &ReducerContext) {
    // Table scans and mutations must be separate passes, even during init.
    let projectile_ids: Vec<_> = ctx.db.projectile().iter().map(|row| row.id).collect();
    let aoe_ids: Vec<_> = ctx.db.aoe_region().iter().map(|row| row.id).collect();
    let crowd_control_ids: Vec<_> = ctx.db.crowd_control().iter().map(|row| row.id).collect();
        let active_status_ids: Vec<_> = ctx.db.active_status().iter().map(|row| row.id).collect();
    let modifier_ids: Vec<_> = ctx.db.stat_modifier().iter().map(|row| row.id).collect();
    let threat_ids: Vec<_> = ctx.db.threat().iter().map(|row| row.id).collect();
    let cast_entity_ids: Vec<_> = ctx
        .db
        .cast_state()
        .iter()
        .map(|row| row.entity_id)
        .collect();
    let cooldown_ids: Vec<_> = ctx.db.cooldown().iter().map(|row| row.id).collect();
    let boss_entity_ids: Vec<_> = ctx
        .db
        .boss_state()
        .iter()
        .map(|row| row.entity_id)
        .collect();
    let seeded_entity_ids: Vec<_> = ctx
        .db
        .game_entity()
        .iter()
        .filter(|row| row.owner.is_none())
        .map(|row| row.entity_id)
        .collect();
    let tick_stat_ids: Vec<_> = ctx.db.tick_stats().iter().map(|row| row.id).collect();

    for id in projectile_ids {
        ctx.db.projectile().id().delete(&id);
    }
    for id in aoe_ids {
        ctx.db.aoe_region().id().delete(&id);
    }
    for id in crowd_control_ids {
        ctx.db.crowd_control().id().delete(&id);
    }
    for id in active_status_ids {
        ctx.db.active_status().id().delete(&id);
    }
    for id in modifier_ids {
        ctx.db.stat_modifier().id().delete(&id);
    }
    for id in threat_ids {
        ctx.db.threat().id().delete(&id);
    }
    for entity_id in cast_entity_ids {
        ctx.db.cast_state().entity_id().delete(&entity_id);
    }
    for id in cooldown_ids {
        ctx.db.cooldown().id().delete(&id);
    }
    for entity_id in boss_entity_ids {
        ctx.db.boss_state().entity_id().delete(&entity_id);
    }
    // Non-player entities are respawned from the map manifest by `world::seed`.
    for entity_id in seeded_entity_ids {
        ctx.db.entity_stats().entity_id().delete(&entity_id);
        ctx.db.game_entity().entity_id().delete(&entity_id);
    }
    for id in tick_stat_ids {
        ctx.db.tick_stats().id().delete(&id);
    }
}

/// Marks an existing character online.
///
/// A connection with no character is normal: the client calls [`join`] once the
/// player has picked a name.
#[reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    let Some(player) = ctx.db.player().identity().find(&ctx.sender()) else {
        return;
    };
    ctx.db.player().identity().update(Player {
        online: true,
        last_seen: ctx.timestamp,
        ..player
    });

    // A returning character arrives with gear already on. `entity_stats` is
    // derived — base plus equipment plus modifiers — and nothing has recomputed
    // it since the equipment last changed, so it is rebuilt here rather than
    // trusted.
    crate::sim::combat::recalculate_effective_stats(ctx, player.entity_id);
}

/// Marks the character offline and stops it where it stands.
///
/// Note what is *not* here: a save. Position, stats and inventory are already
/// rows. The Bevy server wrote its only snapshot at this point, which is why a
/// crash lost the whole session.
#[reducer(client_disconnected)]
pub fn client_disconnected(ctx: &ReducerContext) {
    let Some(player) = ctx.db.player().identity().find(&ctx.sender()) else {
        return;
    };
    if let Some(entity) = ctx.db.game_entity().entity_id().find(&player.entity_id) {
        ctx.db.game_entity().entity_id().update(GameEntity {
            move_target: None,
            state: EntityStateRow::Idle,
            ..entity
        });
    }
    ctx.db.player().identity().update(Player {
        online: false,
        last_seen: ctx.timestamp,
        ..player
    });
}

/// Creates the caller's character, or brings the existing one online.
#[reducer]
pub fn join(ctx: &ReducerContext, display_name: String) -> Result<(), String> {
    let normalized = normalize_name(&display_name);
    if normalized.chars().count() < 3 || normalized.chars().count() > 16 {
        return Err(format!(
            "name must be 3-16 characters, got {display_name:?}"
        ));
    }

    let identity = ctx.sender();
    if let Some(player) = ctx.db.player().identity().find(&identity) {
        ctx.db.player().identity().update(Player {
            online: true,
            last_seen: ctx.timestamp,
            ..player
        });
        return Ok(());
    }

    if ctx
        .db
        .player()
        .normalized_name()
        .find(&normalized)
        .is_some()
    {
        return Err(format!("name {display_name:?} is taken"));
    }

    let spawn = world::player_spawn_point(ctx);
    let (cell_x, cell_z) = grid_cell(spawn);
    let entity = ctx.db.game_entity().insert(GameEntity {
        entity_id: 0,
        kind: EntityKindRow::Player,
        owner: Some(identity),
        display_name: display_name.clone(),
        color: ColorRow::for_kind(EntityKindRow::Player),
        position: spawn,
        look: Vec3Row {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        move_target: None,
        speed: DEFAULT_SPEED_PER_SECOND,
        state: EntityStateRow::Idle,
        cell_x,
        cell_z,
        spawn_point: spawn,
        // Players come back when they ask to, not on a clock.
        respawn_in_seconds: None,
    });

    let defaults = bevymmo_domain::stats::defaults::player_defaults();
    let stats = StatsRow::from(&defaults);
    ctx.db
        .player_stats()
        .insert(PlayerStats { identity, stats });
    ctx.db.entity_stats().insert(crate::tables::EntityStats {
        entity_id: entity.entity_id,
        stats,
        current_mana: stats.max_mana,
    });

    ctx.db.hotbar().insert(Hotbar {
        identity,
        slots: HotbarRow::from(&bevymmo_domain::spells::components::default_player_hotbar()),
    });
    ctx.db.inventory().insert(InventoryTable {
        identity,
        slots: inventory_to_rows(&Default::default()),
    });
    crate::reducers::items::grant_item(ctx, identity, "magic_staff")?;
    ctx.db.equipment().insert(EquipmentTable {
        identity,
        slots: equipment_to_rows(&Default::default()),
    });
    ctx.db.known_glyphs().insert(KnownGlyphsTable {
        identity,
        essences: vec!["fuoco".to_string(), "gelo".to_string(), "terra".to_string()],
        modifiers: vec![
            "espandere".to_string(),
            "amplificare".to_string(),
            "concentrare".to_string(),
        ],
        ancient_words: Vec::new(),
    });
    ctx.db.known_ancient_language().insert(KnownAncientLanguageTable {
        identity,
        root_words: vec!["damage".to_string()],
        ancient_words: vec!["amplia".to_string()],
        base_abilities: vec!["arcane_orb".to_string()],
    });

    ctx.db.player().insert(Player {
        identity,
        normalized_name: normalized,
        display_name,
        entity_id: entity.entity_id,
        online: true,
        last_seen: ctx.timestamp,
    });
    Ok(())
}

/// Permanently deletes the caller's character, freeing its display name.
///
/// The client's identity is a throwaway credential today (see
/// `forget_cached_credentials` in `bevymmo_client::stdb::plugin`): every
/// logout and every shutdown mints a fresh one on the next launch, so an
/// identity that leaves without calling this reducer becomes unreachable
/// forever, and the character it owned squats its name — and briefly its
/// "online" status — with no way for anyone, including its own player, to
/// ever get it back. This is the counterpart to [`join`] that makes that
/// deliberate churn survivable: it removes every row the character owns
/// rather than merely marking it offline, so the name is free the instant
/// this reducer returns.
///
/// A no-op for an identity with no character — logging out from the main
/// menu, before ever calling `join`, is normal.
#[reducer]
pub fn leave(ctx: &ReducerContext) -> Result<(), String> {
    let identity = ctx.sender();
    let Some(player) = ctx.db.player().identity().find(&identity) else {
        return Ok(());
    };
    let entity_id = player.entity_id;

    // Table scans and mutations must be separate passes, as elsewhere in this
    // module.
    let cooldown_ids: Vec<_> = ctx
        .db
        .cooldown()
        .iter()
        .filter(|row| row.entity_id == entity_id)
        .map(|row| row.id)
        .collect();
    let crowd_control_ids: Vec<_> = ctx
        .db
        .crowd_control()
        .iter()
        .filter(|row| row.entity_id == entity_id)
        .map(|row| row.id)
        .collect();
    let active_status_ids: Vec<_> = ctx
        .db
        .active_status()
        .iter()
        .filter(|row| row.entity_id == entity_id)
        .map(|row| row.id)
        .collect();
    let stat_modifier_ids: Vec<_> = ctx
        .db
        .stat_modifier()
        .iter()
        .filter(|row| row.entity_id == entity_id)
        .map(|row| row.id)
        .collect();
    let periodic_effect_ids: Vec<_> = ctx
        .db
        .periodic_effect()
        .iter()
        .filter(|row| row.entity_id == entity_id)
        .map(|row| row.id)
        .collect();
    // A player entity is only ever a threat *target*, never a `boss_entity`.
    let threat_ids: Vec<_> = ctx
        .db
        .threat()
        .iter()
        .filter(|row| row.target_entity == entity_id)
        .map(|row| row.id)
        .collect();
    let resonance_ids: Vec<_> = ctx
        .db
        .resonance()
        .iter()
        .filter(|row| row.identity == identity)
        .map(|row| row.id)
        .collect();

    for id in cooldown_ids {
        ctx.db.cooldown().id().delete(&id);
    }
    for id in crowd_control_ids {
        ctx.db.crowd_control().id().delete(&id);
    }
    for id in active_status_ids {
        ctx.db.active_status().id().delete(&id);
    }
    for id in stat_modifier_ids {
        ctx.db.stat_modifier().id().delete(&id);
    }
    for id in periodic_effect_ids {
        ctx.db.periodic_effect().id().delete(&id);
    }
    for id in threat_ids {
        ctx.db.threat().id().delete(&id);
    }
    for id in resonance_ids {
        ctx.db.resonance().id().delete(&id);
    }

    ctx.db.cast_state().entity_id().delete(&entity_id);
    ctx.db.entity_stats().entity_id().delete(&entity_id);
    ctx.db.game_entity().entity_id().delete(&entity_id);

    ctx.db.known_glyphs().identity().delete(&identity);
    ctx.db.equipment().identity().delete(&identity);
    ctx.db.inventory().identity().delete(&identity);
    ctx.db.hotbar().identity().delete(&identity);
    ctx.db.player_stats().identity().delete(&identity);
    ctx.db.player().identity().delete(&identity);

    Ok(())
}

/// Resolves the caller's entity, or explains why there isn't one.
pub fn caller_entity(ctx: &ReducerContext) -> Result<GameEntity, String> {
    let player = ctx
        .db
        .player()
        .identity()
        .find(&ctx.sender())
        .ok_or_else(|| "no character for this identity; call `join` first".to_string())?;
    ctx.db
        .game_entity()
        .entity_id()
        .find(&player.entity_id)
        .ok_or_else(|| "character has no entity".to_string())
}

/// How long a character stays "online" without hearing from its client.
///
/// Presence cannot be read from the database: the module has no way to
/// enumerate live connections, and `client_disconnected` does not fire for
/// connections that died with a previous instance of the server. So it is
/// inferred from a heartbeat instead. Generous enough to survive a slow frame,
/// short enough that a restarted server does not show a lobby full of ghosts.
const PRESENCE_TIMEOUT_SECONDS: i64 = 15;

/// Says the caller is still here. The client calls this every few seconds.
#[reducer]
pub fn heartbeat(ctx: &ReducerContext) -> Result<(), String> {
    let player = ctx
        .db
        .player()
        .identity()
        .find(&ctx.sender())
        .ok_or_else(|| "no character for this identity".to_string())?;
    ctx.db.player().identity().update(Player {
        online: true,
        last_seen: ctx.timestamp,
        ..player
    });
    Ok(())
}

/// Marks characters offline once their client stops checking in.
///
/// Called from the tick. Note that this is what makes a server restart settle
/// correctly: the tick resumes, but nothing refreshes `last_seen`, so every
/// character that was online when the instance died decays within the timeout.
pub fn expire_stale_presence(ctx: &ReducerContext) {
    let now = ctx.timestamp;
    let stale: Vec<_> = ctx
        .db
        .player()
        .iter()
        .filter(|player| player.online)
        .filter(|player| {
            now.duration_since(player.last_seen)
                .map(|elapsed| elapsed.as_secs() as i64 >= PRESENCE_TIMEOUT_SECONDS)
                // A `last_seen` in the future means clock weirdness, not
                // absence; leave those alone rather than kicking them.
                .unwrap_or(false)
        })
        .collect();

    for player in stale {
        log::info!("{} timed out", player.display_name);
        ctx.db.player().identity().update(Player {
            online: false,
            ..player
        });
    }
}
