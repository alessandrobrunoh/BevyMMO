//! Crowd control: timers, expiry, and the gates the rest of the simulation asks.
//!
//! Ported from `crates/server/src/crowd_control/systems.rs`, which was three
//! Bevy systems — `apply_cc_events`, `tick_crowd_control` and
//! `cancel_active_casts_on_block`. Here they collapse into one [`step`] plus two
//! entry points other modules call directly ([`apply`], [`has_blocking_cc`]),
//! because the event/queue indirection Bevy needed to avoid overlapping
//! `Query` borrows has no reason to exist inside a single transaction.
//!
//! # Why the domain type is not stored
//!
//! `bevymmo_domain::crowd_control::CrowdControlState` is the rulebook and the
//! rules below follow it exactly (refresh instead of stack; expire at zero;
//! blocking kinds suppress action). The *type* is not reused because it cannot
//! represent the schema: `CrowdControlKind` only has `Stun`, while the
//! `crowd_control` table already carries `Root`, `Silence` and `Slow`. Rather
//! than store a lossy projection, each row keeps its own kind and the
//! predicates below extend the domain rule to the three kinds the domain enum
//! has not caught up with yet. `CrowdControlKind::is_blocking` is still the
//! authority for `Stun`, so the two cannot silently disagree about it.
//!
//! # One row per effect, not one component per entity
//!
//! Bevy kept a `CrowdControlState` component with a `Vec` of effects, and left
//! it attached (empty) after expiry to avoid insert/remove churn. Rows are not
//! components: an empty row is a row that still has to be scanned every tick, so
//! expired effects are deleted outright and "no CC" is simply "no rows".

use bevymmo_domain::crowd_control::CrowdControlKind;
use spacetimedb::{ReducerContext, Table};

use crate::tables::{
    cast_state, crowd_control, game_entity, CrowdControl, CrowdControlKindRow, GameEntity,
};

/// Advances every CC timer, drops what expired, and enforces the two things a
/// blocking effect must do: stop the victim casting, and stop it walking.
///
/// Runs first in the tick (see `crate::tick`), which is what makes the movement
/// freeze exact rather than one tick late: `sim::movement::step` reads
/// `move_target` immediately after this, and `sim::ai::step` only re-issues
/// destinations at the end of the tick.
pub fn step(ctx: &ReducerContext, dt: f32) {
    // Two passes, never one: the loop below reads the table it is about to
    // write, and a reducer may not mutate a table it is iterating.
    let mut ticked: Vec<CrowdControl> = Vec::new();
    let mut expired: Vec<u64> = Vec::new();
    let mut casting_blocked: Vec<u64> = Vec::new();
    let mut movement_blocked: Vec<u64> = Vec::new();

    for effect in ctx.db.crowd_control().iter() {
        let remaining = effect.remaining_seconds - dt;
        if remaining <= 0.0 {
            // An effect that ends on this tick no longer gates anything, which
            // is also how the Bevy order behaved: `tick_crowd_control` ran
            // before `cancel_active_casts_on_block`.
            expired.push(effect.id);
            continue;
        }
        if blocks_casting(effect.kind) {
            casting_blocked.push(effect.entity_id);
        }
        if blocks_movement(effect.kind) {
            movement_blocked.push(effect.entity_id);
        }
        ticked.push(CrowdControl {
            remaining_seconds: remaining,
            ..effect
        });
    }

    for effect in ticked {
        ctx.db.crowd_control().id().update(effect);
    }
    for id in expired {
        ctx.db.crowd_control().id().delete(id);
    }

    // Duplicates are harmless: the second visit finds nothing left to cancel.
    for entity_id in casting_blocked {
        cancel_cast(ctx, entity_id);
    }
    for entity_id in movement_blocked {
        freeze(ctx, entity_id);
    }
}

/// Applies `kind` to `entity_id` for `duration_seconds`, refreshing instead of
/// stacking.
///
/// The entry point every CC source uses — spells, AoE regions, traps. Bevy
/// funnelled these through `ApplyCrowdControlEvent` and *dropped* the first
/// event against a target that had no `CrowdControlState` yet, because the
/// component was inserted through `Commands` and only became visible next
/// frame. A row insert is visible immediately, so no application is lost here.
///
/// Refresh semantics match `CrowdControlState::apply`: the new duration
/// replaces the old one outright, even when it is shorter. That keeps a chain
/// of stuns bounded, which was the point of the rule.
pub fn apply(
    ctx: &ReducerContext,
    entity_id: u64,
    kind: CrowdControlKindRow,
    duration_seconds: f32,
) {
    if duration_seconds <= 0.0 {
        return;
    }
    let existing = ctx
        .db
        .crowd_control()
        .victim()
        .filter(entity_id)
        .find(|effect| effect.kind == kind);

    match existing {
        Some(effect) => {
            ctx.db.crowd_control().id().update(CrowdControl {
                remaining_seconds: duration_seconds,
                ..effect
            });
        }
        None => {
            ctx.db.crowd_control().insert(CrowdControl {
                // Zero asks the sequence for an id.
                id: 0,
                entity_id,
                kind,
                remaining_seconds: duration_seconds,
            });
        }
    }

    if blocks_casting(kind) {
        cancel_cast(ctx, entity_id);
    }
    if blocks_movement(kind) {
        freeze(ctx, entity_id);
    }
}

/// Whether `entity_id` is under an effect that suppresses *all* action.
///
/// The predicate `CrowdControlState::has_blocking_cc` answered, and the one
/// callers should use when they mean "this entity cannot act at all". For the
/// narrower questions prefer [`is_casting_blocked`] or [`is_movement_blocked`]:
/// a silenced entity can still walk, and a rooted one can still cast.
pub fn has_blocking_cc(ctx: &ReducerContext, entity_id: u64) -> bool {
    ctx.db
        .crowd_control()
        .victim()
        .filter(entity_id)
        .any(|effect| blocks_all_actions(effect.kind))
}

/// Whether `entity_id` may not start or continue a cast.
pub fn is_casting_blocked(ctx: &ReducerContext, entity_id: u64) -> bool {
    ctx.db
        .crowd_control()
        .victim()
        .filter(entity_id)
        .any(|effect| blocks_casting(effect.kind))
}

/// Whether `entity_id` may not move under its own power.
pub fn is_movement_blocked(ctx: &ReducerContext, entity_id: u64) -> bool {
    ctx.db
        .crowd_control()
        .victim()
        .filter(entity_id)
        .any(|effect| blocks_movement(effect.kind))
}

/// Cancels an in-progress cast, if there is one.
///
/// Bevy removed `CastProgress` silently and let the cast bar UI time itself out
/// after roughly a second. `sim::spells::end_cast` emits a `cast_ended` event
/// instead, so the interrupt is stated rather than inferred — a second of stale
/// cast bar on a stunned character is exactly the moment the player is looking
/// at it. Ending a cast belongs to the cast pipeline, so that is what does it;
/// this only decides *when*.
fn cancel_cast(ctx: &ReducerContext, entity_id: u64) {
    let Some(cast) = ctx.db.cast_state().entity_id().find(entity_id) else {
        return;
    };
    crate::sim::spells::end_cast(ctx, entity_id, cast.spell_id, true);
}

/// Drops whatever destination an entity was walking to.
///
/// Bevy gated the movement *system* on the absence of blocking CC. There is no
/// system to gate here — `sim::movement::step` walks whatever `move_target`
/// says — so the destination is cleared instead. The visible difference is that
/// a stunned character does not resume its interrupted path when the stun ends;
/// it stands still until it is told where to go again, which is what a stun is
/// supposed to feel like.
fn freeze(ctx: &ReducerContext, entity_id: u64) {
    let Some(entity) = ctx.db.game_entity().entity_id().find(entity_id) else {
        return;
    };
    if entity.move_target.is_none() {
        // Already stopped; skip the write so a long stun costs one row update
        // rather than one per tick.
        return;
    }
    ctx.db.game_entity().entity_id().update(GameEntity {
        move_target: None,
        ..entity
    });
}

/// Whether this kind suppresses every action, movement and casting alike.
///
/// `Stun` defers to `CrowdControlKind::is_blocking` so the module and the
/// domain cannot drift on the one kind they both know about.
fn blocks_all_actions(kind: CrowdControlKindRow) -> bool {
    match kind {
        CrowdControlKindRow::Stun => CrowdControlKind::Stun.is_blocking(),
        // Root, Silence and Slow each suppress one axis at most.
        CrowdControlKindRow::Root | CrowdControlKindRow::Silence | CrowdControlKindRow::Slow => {
            false
        }
    }
}

/// Whether this kind stops the victim casting.
fn blocks_casting(kind: CrowdControlKindRow) -> bool {
    match kind {
        CrowdControlKindRow::Stun => blocks_all_actions(kind),
        CrowdControlKindRow::Silence => true,
        CrowdControlKindRow::Root | CrowdControlKindRow::Slow => false,
    }
}

/// Whether this kind stops the victim moving.
///
/// `Slow` is deliberately absent: the domain's own note says a slow belongs in
/// the stat pipeline as a `movement_speed` modifier, not in the CC gate. It
/// stays in the table so the UI can show it and so immunity rules can see it.
fn blocks_movement(kind: CrowdControlKindRow) -> bool {
    match kind {
        CrowdControlKindRow::Stun => blocks_all_actions(kind),
        CrowdControlKindRow::Root => true,
        CrowdControlKindRow::Silence | CrowdControlKindRow::Slow => false,
    }
}
