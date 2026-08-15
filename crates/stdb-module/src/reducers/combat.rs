//! What a client is allowed to ask of the combat system.
//!
//! Only respawn, for now. Damage and healing are never client-requested: they
//! are consequences the tick produces, and the entry points live in
//! [`crate::sim::combat`].

use spacetimedb::{reducer, ReducerContext, Table};

use crate::reducers::lifecycle::caller_entity;
use crate::sim::combat;
use crate::tables::{
    crowd_control, entity_stats, game_entity, grid_cell, player_message, EntityStateRow,
    EntityStats, GameEntity, PlayerMessageEvent,
};

/// Brings the caller's character back to life at its spawn point.
///
/// Ported from `handle_respawn_requests`. Two things the Bevy version needed
/// and this does not: it had to scan every player entity to find the one whose
/// `PlayerId` matched the requesting peer — `ctx.sender()` answers that with no
/// scan and no way to spoof — and it had to pick a spawn point from a shared
/// pool because only *some* players carried a `SpawnPoint`. Every
/// `game_entity` has a `spawn_point` column, so the fallback is gone.
///
/// Refusing when the character is alive is deliberate: `RespawnRequest` was a
/// payload-free message, so a live player spamming it would otherwise get a
/// free full heal and a teleport home.
#[reducer]
pub fn respawn(ctx: &ReducerContext) -> Result<(), String> {
    let entity = caller_entity(ctx)?;
    let stats = ctx
        .db
        .entity_stats()
        .entity_id()
        .find(&entity.entity_id)
        .ok_or_else(|| "character has no stats".to_string())?;

    // Both conditions, as in the Bevy server: zero health counts as dead even
    // if the state has not caught up yet. The tick's death sweep normally makes
    // them agree, but a respawn arriving in the same tick as the killing blow
    // should not be rejected on a technicality.
    if entity.state != EntityStateRow::Dead && stats.stats.current_health > 0.0 {
        return Err("only dead characters respawn".to_string());
    }

    // Order matters: dropping the modifiers first restores the real
    // `max_health`, so the refill below tops up to the unbuffed pool rather
    // than to a number a debuff had shrunk.
    combat::clear_modifiers(ctx, entity.entity_id);
    clear_crowd_control(ctx, entity.entity_id);

    // Re-read: `clear_modifiers` rewrites this row.
    let stats = ctx
        .db
        .entity_stats()
        .entity_id()
        .find(&entity.entity_id)
        .ok_or_else(|| "character has no stats".to_string())?;
    let refilled = crate::rows::StatsRow {
        current_health: stats.stats.max_health,
        ..stats.stats
    };
    ctx.db.entity_stats().entity_id().update(EntityStats {
        stats: refilled,
        current_mana: refilled.max_mana,
        ..stats
    });

    let position = entity.spawn_point;
    let (cell_x, cell_z) = grid_cell(position);
    ctx.db.game_entity().entity_id().update(GameEntity {
        position,
        // Whatever the character was walking towards when it died is not where
        // it wants to go from the graveyard.
        move_target: None,
        state: EntityStateRow::Idle,
        cell_x,
        cell_z,
        ..entity
    });

    ctx.db.player_message().insert(PlayerMessageEvent {
        target: Some(ctx.sender()),
        text: "You are back on your feet.".to_string(),
    });
    Ok(())
}

/// Drops every stun, root, silence and slow on an entity.
///
/// Respawning out of a stun is the point: the crowd control that killed the
/// character should not still be running when it stands back up.
fn clear_crowd_control(ctx: &ReducerContext, entity_id: u64) {
    // Collected first: deleting while the index iterator is live is not safe.
    let ids: Vec<u64> = ctx
        .db
        .crowd_control()
        .victim()
        .filter(&entity_id)
        .map(|row| row.id)
        .collect();
    for id in ids {
        ctx.db.crowd_control().id().delete(&id);
    }
}
