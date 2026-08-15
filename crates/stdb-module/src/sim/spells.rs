//! The spell runtime: casts in progress, projectiles in flight, AoE regions on
//! the ground, and the cooldowns that gate them all.
//!
//! This is the port of `crates/server/src/spells` (`systems.rs`, `projectile.rs`,
//! `aoe.rs`). The game rules did *not* come with it: `Spell::cast` is the same
//! function that ran under Bevy, because [`SpellCastContext`] was already a pure
//! collector of pending effects with no ECS in it. What lives here is only the
//! part Bevy used to provide — where the caster is, who is nearby, and how a
//! pending effect becomes a row.
//!
//! # What changed, and why
//!
//! - **No `replicate_cast_progress`.** `cast_state` is a public table, so the
//!   client subscribes to the cast it wants to draw instead of being sent a
//!   snapshot every 100 ms.
//! - **Movement interrupts measure from the *start* of the cast**, not from the
//!   previous tick: `cast_state.start_position` is written once and never moved.
//!   Bevy compared against `last_position`, which let a caster drift forever as
//!   long as each single tick stayed under the epsilon.
//! - **No input snapshot.** Bevy also cancelled on a *new movement command*,
//!   because a click could be issued before the character had moved. Here the
//!   click writes `game_entity.move_target` and the character starts moving on
//!   the very next tick, so position alone catches it one tick later.
//! - **Crowd control cancels a running cast.** Bevy only checked CC when a cast
//!   *started*; a stun landing mid-cast left the wind-up running.
//! - **Projectiles carry the id of the spell that fired them.** The Bevy spawner
//!   was literally `spell_id: "fireball".to_string()` regardless of the caster.

use std::sync::OnceLock;

use bevymmo_domain::abilities::{
    AncientWordRegistry, BaseAbilityRegistry, EssenceRegistry, ModifierRegistry,
};
use bevymmo_domain::crowd_control::CrowdControlKind;
use bevymmo_domain::items::registry::ItemRegistry;
use bevymmo_domain::spells::components::MOVEMENT_INTERRUPT_EPSILON;
use bevymmo_domain::spells::context::{
    AoeEffect, AoeShape, AoeSpawnRequest, AoeTargeting, CastKind, ChannelMovementPolicy,
    ProjectileSpawnRequest, Spell, SpellCastContext,
};
use bevymmo_domain::spells::registry::{SpellId, SpellRegistry};
use bevymmo_domain::stats::components::{CombatStats, StatsBundleData};
use bevymmo_domain::stats::events::{ApplyStatModifierEvent, ModifierEffect, ModifierOp};
use bevymmo_domain::EntityId;
use glam::Vec3;
use spacetimedb::{ReducerContext, Table};

use crate::rows::Vec3Row;
use crate::tables::{
    aoe_region, cast_ended, cast_state, cooldown, entity_stats, game_entity,
    grid_cell, projectile, spell_visual_effect, AoeRegion, AoeShapeRow, CastEndedEvent,
    CastKindRow, CastState, Cooldown, CrowdControlKindRow, EntityStateRow,
    GameEntity, Projectile, SpellVisualEffectEvent,
};

/// How long a projectile may stay in the air before it gives up, in seconds.
///
/// The Bevy version had no lifetime at all: a projectile lived until its target
/// died or despawned. That is not safe here, where "the projectile entity" is a
/// persisted row — a target that simply outruns it forever would leak a row per
/// cast. Generous enough that no spell in the registry can reach it (the fastest
/// projectile covers 240 units in this window, against a 15-unit cast range).
const PROJECTILE_MAX_LIFETIME_SECONDS: f32 = 10.0;

/// Slack added to the spatial query that builds `potential_targets`.
///
/// The cell scan is centred on the caster with a radius of
/// `cast_range + area_radius`; the margin covers projectile hit radii and the
/// half-second of movement a target can manage between the client aiming and
/// the reducer running.
pub const TARGET_QUERY_MARGIN: f32 = 4.0;

/// Slack allowed on the server-side range check, in world units.
///
/// The client aims against its own extrapolated position, so a cast issued
/// exactly at the limit arrives a few centimetres long. Rejecting those would
/// make max-range casting feel broken.
pub const CAST_RANGE_TOLERANCE: f32 = 1.0;

// ---------------------------------------------------------------------------
// Registries
// ---------------------------------------------------------------------------
//
// Built once per module instance, not per cast: `default_spells` allocates an
// `Arc` per spell and a `HashMap`, and a cast happens several times a second
// per player. `OnceLock` rather than `lazy_static` because the module is
// single-threaded and this needs no dependency.

pub fn spells() -> &'static SpellRegistry {
    static REGISTRY: OnceLock<SpellRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::spells_impl::default_spells)
}

pub fn base_abilities() -> &'static BaseAbilityRegistry {
    static REGISTRY: OnceLock<BaseAbilityRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::base_abilities_impl::default_base_abilities)
}

pub fn essences() -> &'static EssenceRegistry {
    static REGISTRY: OnceLock<EssenceRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::essences_impl::default_essences)
}

pub fn modifiers() -> &'static ModifierRegistry {
    static REGISTRY: OnceLock<ModifierRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::modifiers_impl::default_modifiers)
}

pub fn ancient_words() -> &'static AncientWordRegistry {
    static REGISTRY: OnceLock<AncientWordRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::ancient_words_impl::default_ancient_words)
}

/// The item catalogue, needed to read the equipped weapon's Eidolon gestures.
///
/// Lives here rather than in `reducers::items` because the spell path is its
/// only consumer today; move it if the inventory reducers grow one.
pub fn items() -> &'static ItemRegistry {
    static REGISTRY: OnceLock<ItemRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::items_impl::default_items)
}

// ---------------------------------------------------------------------------
// Queries shared with the reducers
// ---------------------------------------------------------------------------

/// The caster's combat stats, or `None` if it has no stats row.
pub fn combat_stats(ctx: &ReducerContext, entity_id: u64) -> Option<CombatStats> {
    let row = ctx.db.entity_stats().entity_id().find(&entity_id)?;
    Some(StatsBundleData::from(row.stats).combat)
}

/// Whether an entity can still be hit: it exists, is not flagged dead, and has
/// health left. Mirrors Bevy's `!vital.is_dead()` filter on target queries.
fn is_alive(ctx: &ReducerContext, entity: &GameEntity) -> bool {
    if entity.state == EntityStateRow::Dead {
        return false;
    }
    ctx.db
        .entity_stats()
        .entity_id()
        .find(&entity.entity_id)
        .is_none_or(|stats| stats.stats.current_health > 0.0)
}

/// Every living entity within `radius` of `center`, as `Spell::cast` wants them.
///
/// Bevy handed each cast *every* `GameEntity` in the world and let the spell
/// filter; that is a full table scan per cast here, so the candidates come from
/// the `cell_x`/`cell_z` index instead. The result is still a superset of what
/// any single spell will use — the spell's own radius/cone/line test runs on top
/// of it, unchanged.
pub fn potential_targets(ctx: &ReducerContext, center: Vec3, radius: f32) -> Vec<(EntityId, Vec3)> {
    let radius = radius.max(0.0);
    let (min_x, min_z) = grid_cell(Vec3Row {
        x: center.x - radius,
        y: 0.0,
        z: center.z - radius,
    });
    let (max_x, max_z) = grid_cell(Vec3Row {
        x: center.x + radius,
        y: 0.0,
        z: center.z + radius,
    });

    let mut targets = Vec::new();
    for cell_x in min_x..=max_x {
        // The index is `(cell_x, cell_z)`, so the scan fixes the first column
        // and ranges over the second — one syscall per column of cells.
        for entity in ctx.db.game_entity().cell().filter((cell_x, min_z..=max_z)) {
            if !is_alive(ctx, &entity) {
                continue;
            }
            let position = Vec3::from(entity.position);
            if flat_distance(center, position) > radius {
                continue;
            }
            targets.push((EntityId::new(entity.entity_id), position));
        }
    }
    targets
}

/// Horizontal distance. Height is discarded everywhere in this game's maths
/// (see `AoeShape::contains`), because combat happens on a plane.
pub fn flat_distance(from: Vec3, to: Vec3) -> f32 {
    Vec3::new(to.x - from.x, 0.0, to.z - from.z).length()
}

/// Whether a crowd control effect currently prevents this entity from casting.
///
/// The domain's `CrowdControlKind` only knows `Stun`, so it cannot classify the
/// `Silence` the row enum carries; the predicate lives here until the two enums
/// agree. `Root` and `Slow` deliberately do not block casting — they are
/// movement effects.
pub fn casting_blocked(ctx: &ReducerContext, entity_id: u64) -> bool {
    // Delegates so the two cannot drift on which effects gag a caster.
    crate::sim::crowd_control::is_casting_blocked(ctx, entity_id)
}

/// Whether `ability_id` (a spell id or an Eidolon gesture id) is still cooling
/// down for this entity.
pub fn is_on_cooldown(ctx: &ReducerContext, entity_id: u64, ability_id: &str) -> bool {
    ctx.db
        .cooldown()
        .owner_ability()
        .filter((entity_id, ability_id))
        .any(|row| row.elapsed_seconds < row.duration_seconds)
}

/// Puts `ability_id` on cooldown, replacing any existing timer for it.
///
/// Replacing rather than adding mirrors `SpellCooldowns::start_cooldown`, which
/// inserted into a map keyed by id: a second cast can only ever refresh.
pub fn start_cooldown(ctx: &ReducerContext, entity_id: u64, ability_id: &str, duration: f32) {
    if duration <= 0.0 {
        return;
    }
    let existing: Vec<u64> = ctx
        .db
        .cooldown()
        .owner_ability()
        .filter((entity_id, ability_id))
        .map(|row| row.id)
        .collect();
    for id in existing {
        ctx.db.cooldown().id().delete(&id);
    }
    ctx.db.cooldown().insert(Cooldown {
        id: 0,
        entity_id,
        ability_id: ability_id.to_string(),
        elapsed_seconds: 0.0,
        duration_seconds: duration,
    });
}

/// Ends whatever `entity_id` is casting, telling subscribers how it ended.
pub fn end_cast(ctx: &ReducerContext, entity_id: u64, spell_id: String, interrupted: bool) {
    ctx.db.cast_state().entity_id().delete(&entity_id);
    ctx.db.cast_ended().insert(CastEndedEvent {
        entity_id,
        spell_id,
        interrupted,
    });
}

// ---------------------------------------------------------------------------
// Firing a spell
// ---------------------------------------------------------------------------

/// Builds the cast context, runs `Spell::cast`, and turns the pending effects
/// into rows. Returns the cooldown the caster earned, or `None` if the cast
/// could not run at all.
///
/// The direct port of Bevy's `fire_spell`, and the only place a spell's own
/// logic is invoked — the instant path, the cast-time completion and every
/// channel tick all come through here.
pub fn fire_spell(
    ctx: &ReducerContext,
    caster: &GameEntity,
    spell: &dyn Spell,
    target_position: Option<Vec3>,
    target_entity: Option<u64>,
) -> Option<f32> {
    let combat = combat_stats(ctx, caster.entity_id)?;
    let config = spell.config();
    let caster_position = Vec3::from(caster.position);

    let query_radius = config.cast_range + config.area_radius + TARGET_QUERY_MARGIN;
    let targets = potential_targets(ctx, caster_position, query_radius);

    let mut cast = SpellCastContext::new(
        EntityId::new(caster.entity_id),
        caster_position,
        &combat,
        Vec3::from(caster.look),
        target_position,
        target_entity.map(EntityId::new),
        &targets,
    );

    spell.cast(&mut cast);
    apply_pending(
        ctx,
        caster.entity_id,
        caster_position,
        spell.id().as_str(),
        &mut cast,
    );

    Some(config.cooldown_seconds)
}

/// Drains every `pending_*` list on the context into the database.
///
/// Bevy's `apply_spell_effects`, with message writers replaced by table writes.
/// `spell_id` is the id of whatever produced the effects — a spell for the
/// classic path, an Eidolon gesture for `eidolon_cast` — and is what a spawned
/// projectile or region is labelled with.
pub fn apply_pending(
    ctx: &ReducerContext,
    caster: u64,
    caster_position: Vec3,
    spell_id: &str,
    cast: &mut SpellCastContext,
) {
    for event in cast.pending_damage.drain(..) {
        crate::sim::combat::apply_damage(ctx, event.target.get(), Some(caster), event.amount);
    }
    for event in cast.pending_healing.drain(..) {
        crate::sim::combat::apply_healing(ctx, event.target.get(), event.amount);
    }
    for request in cast.pending_projectiles.drain(..) {
        spawn_projectile(ctx, caster, caster_position, spell_id, request);
    }
    for request in cast.pending_aoes.drain(..) {
        spawn_aoe_region(ctx, caster, request);
    }
    for event in cast.pending_modifiers.drain(..) {
        apply_modifier_event(ctx, &event);
    }
    for visual in cast.pending_visuals.drain(..) {
        ctx.db.spell_visual_effect().insert(SpellVisualEffectEvent {
            spell_id: visual.spell_id,
            start: visual.start.into(),
            end: visual.end.into(),
        });
    }
}

/// One homing projectile, as a row.
fn spawn_projectile(
    ctx: &ReducerContext,
    caster: u64,
    start: Vec3,
    spell_id: &str,
    request: ProjectileSpawnRequest,
) {
    ctx.db.projectile().insert(Projectile {
        id: 0,
        caster,
        spell_id: spell_id.to_string(),
        position: start.into(),
        target_entity: Some(request.target.get()),
        target_position: None,
        speed: request.speed,
        damage: request.damage,
        hit_radius: request.hit_radius,
        remaining_seconds: PROJECTILE_MAX_LIFETIME_SECONDS,
    });
}

/// Translates one `ApplyStatModifierEvent` into `stat_modifier` rows.
///
/// Split by effect kind because the two land in different tables: a stat change
/// goes to `stat_modifier` and is folded into the effective stats, while a
/// periodic heal or damage goes to `periodic_effect` and changes health on a
/// schedule instead. `ModifierOp::Override` still has nowhere to go —
/// `stat_modifier` carries a bool, not an operation — and is dropped with a
/// warning rather than approximated.
fn apply_modifier_event(ctx: &ReducerContext, event: &ApplyStatModifierEvent) {
    for effect in &event.effects {
        match effect {
            ModifierEffect::Stat {
                field,
                operation,
                value,
            } => {
                let is_multiplicative = match operation {
                    ModifierOp::Add => false,
                    ModifierOp::Multiply => true,
                    ModifierOp::Override => {
                        log::warn!(
                            "dropping an Override modifier on {}: `stat_modifier` has no column \
                             for it",
                            event.target
                        );
                        continue;
                    }
                };
                crate::sim::combat::apply_modifier(
                    ctx,
                    event.target.get(),
                    &format!("{field:?}"),
                    *value,
                    is_multiplicative,
                    event.duration_seconds,
                );
            }
            ModifierEffect::HealOverTime {
                amount_per_tick,
                tick_interval,
            } => crate::sim::combat::apply_periodic(
                ctx,
                event.target.get(),
                event.source.map(|s| s.get()),
                *amount_per_tick,
                *tick_interval,
                // A periodic effect with no duration would never stop. The
                // domain allows it; the table would keep ticking forever, so it
                // is treated as a no-op rather than a leak.
                event.duration_seconds.unwrap_or(0.0),
            ),
            ModifierEffect::DamageOverTime {
                amount_per_tick,
                tick_interval,
            } => crate::sim::combat::apply_periodic(
                ctx,
                event.target.get(),
                event.source.map(|s| s.get()),
                // Negative heals: `apply_periodic` takes one signed number.
                -amount_per_tick.abs(),
                *tick_interval,
                event.duration_seconds.unwrap_or(0.0),
            ),
        }
    }
}

/// Translates a domain crowd control kind into its row form.
///
/// `CrowdControlKind` currently has one variant. The row enum has four because
/// the schema anticipates the rest; until the domain grows them, no spell can
/// produce a Root, Silence or Slow, and that is a content gap rather than a
/// porting one.
fn crowd_control_row_kind(kind: CrowdControlKind) -> CrowdControlKindRow {
    match kind {
        CrowdControlKind::Stun => CrowdControlKindRow::Stun,
    }
}

// ---------------------------------------------------------------------------
// AoE regions
// ---------------------------------------------------------------------------

/// Spawns a requested AoE region, or applies it on the spot.
///
/// `aoe_region` can carry a burst of damage or healing over a circle, and
/// nothing else: there is no column for a cone's aperture, for `AoeTargeting`,
/// for a crowd control payload or for a stat modifier payload. A request the
/// table cannot hold faithfully is therefore resolved *now*, against whoever is
/// inside the shape at cast time, using the domain's own `AoeShape::contains`
/// and `AoeTargeting::allows`.
///
/// That trade is deliberate. The alternative — dropping what does not fit —
/// turns Stun Field, Binding Seal, Wing Buffet and Healing Circle into no-ops,
/// which is further from the Bevy server than losing a wind-up. What is lost is
/// the telegraph delay and the "someone walks in later" case; see the report on
/// the columns `aoe_region` still needs.
fn spawn_aoe_region(ctx: &ReducerContext, caster: u64, request: AoeSpawnRequest) {
    let Some(row) = persistable_region(caster, &request) else {
        apply_aoe_now(ctx, caster, &request);
        return;
    };
    ctx.db.aoe_region().insert(row);
}

/// The row for `request`, or `None` when the table would misrepresent it.
fn persistable_region(caster: u64, request: &AoeSpawnRequest) -> Option<AoeRegion> {
    // A region with no lifetime would be applied and despawned on the tick
    // after it spawned, so resolving it at cast time is the same thing one tick
    // earlier — and saves a row round-trip for every melee swing.
    if request.duration_seconds <= 0.0 {
        return None;
    }
    // Only a circle survives storage: `AoeShapeRow::Cone` has a `direction` but
    // no aperture, and guessing one would silently resize the hitbox.
    let AoeShape::Circle = request.shape else {
        return None;
    };
    let (damage, healing) = match &request.effect {
        AoeEffect::Damage { amount, .. } => (*amount, 0.0),
        AoeEffect::Heal { amount, .. } => (0.0, *amount),
        AoeEffect::ApplyModifier { .. } | AoeEffect::CrowdControl { .. } => return None,
    };
    // `affected` exists to make an effect apply once per entity, which is
    // exactly the semantics Bevy gave burst damage and healing. Seeding it with
    // the caster is also how `AoeTargeting::ExcludeCaster` survives a schema
    // with no targeting column: the caster is simply "already affected".
    let affected = match request.effect.targeting() {
        AoeTargeting::Everyone => Vec::new(),
        AoeTargeting::ExcludeCaster => vec![caster],
        // Nothing else can be expressed by pre-seeding, so it takes the
        // immediate path where the policy is applied in full.
        AoeTargeting::CasterOnly => return None,
    };

    Some(AoeRegion {
        id: 0,
        caster,
        spell_id: request.spell_id.clone(),
        center: request.center.into(),
        direction: Vec3Row::default(),
        radius: request.radius,
        shape: AoeShapeRow::Circle,
        remaining_seconds: request.duration_seconds,
        pending_delay_seconds: request.initial_delay_seconds.max(0.0),
        affected,
        damage,
        healing,
    })
}

/// Applies an AoE request immediately to everything currently inside it.
fn apply_aoe_now(ctx: &ReducerContext, caster: u64, request: &AoeSpawnRequest) {
    let targeting = request.effect.targeting();
    let caster_id = EntityId::new(caster);
    let inside: Vec<EntityId> = potential_targets(ctx, request.center, request.radius)
        .into_iter()
        .filter(|(target, position)| {
            targeting.allows(caster_id, *target)
                && request
                    .shape
                    .contains(request.center, request.radius, *position)
        })
        .map(|(target, _)| target)
        .collect();

    for target in inside {
        match &request.effect {
            AoeEffect::Damage { amount, .. } => {
                crate::sim::combat::apply_damage(ctx, target.get(), Some(caster), *amount)
            }
            AoeEffect::Heal { amount, .. } => {
                crate::sim::combat::apply_healing(ctx, target.get(), *amount)
            }
            AoeEffect::CrowdControl {
                kind,
                duration_seconds,
                ..
            } => crate::sim::crowd_control::apply(
                ctx,
                target.get(),
                crowd_control_row_kind(*kind),
                *duration_seconds,
            ),
            AoeEffect::ApplyModifier {
                effects,
                duration_seconds,
                kind,
                ..
            } => apply_modifier_event(
                ctx,
                &ApplyStatModifierEvent {
                    target,
                    source: Some(caster_id),
                    effects: effects.clone(),
                    duration_seconds: *duration_seconds,
                    kind: *kind,
                },
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// The tick
// ---------------------------------------------------------------------------

/// One simulation step of the spell system.
///
/// Same order as the Bevy `.chain()`: casts advance (and may fire), then what
/// they spawned moves, then cooldowns tick. Anything the fire produced this tick
/// therefore waits for the next one before it acts, exactly as before.
pub fn step(ctx: &ReducerContext, dt: f32) {
    advance_casts(ctx, dt);
    update_projectiles(ctx, dt);
    update_aoe_regions(ctx, dt);
    tick_cooldowns(ctx, dt);
}

/// A cast that will not survive this tick.
struct EndedCast {
    entity_id: u64,
    spell_id: String,
    interrupted: bool,
}

/// Bevy's `advance_cast_progress`: ticks every wind-up and channel, fires the
/// ones that came due, and cancels the ones that were interrupted.
fn advance_casts(ctx: &ReducerContext, dt: f32) {
    // Collected up front because firing writes to `entity_stats`, `projectile`,
    // `aoe_region` and `cooldown`, and a tick is one transaction: iterating a
    // table while the same transaction writes it is not something to rely on.
    let casts: Vec<CastState> = ctx.db.cast_state().iter().collect();
    let mut ended: Vec<EndedCast> = Vec::new();

    for cast in casts {
        let Some(caster) = ctx.db.game_entity().entity_id().find(&cast.entity_id) else {
            // The caster was removed between starting the cast and this tick.
            ended.push(EndedCast {
                entity_id: cast.entity_id,
                spell_id: cast.spell_id,
                interrupted: true,
            });
            continue;
        };

        if !is_alive(ctx, &caster) || casting_blocked(ctx, caster.entity_id) {
            ended.push(EndedCast {
                entity_id: cast.entity_id,
                spell_id: cast.spell_id,
                interrupted: true,
            });
            continue;
        }

        let Some(spell) = spells().get(&SpellId::new(cast.spell_id.clone())) else {
            log::warn!(
                "cast in progress for unknown spell {:?}; cancelling",
                cast.spell_id
            );
            ended.push(EndedCast {
                entity_id: cast.entity_id,
                spell_id: cast.spell_id,
                interrupted: true,
            });
            continue;
        };
        let config = spell.config();

        let movement_cancels = match cast.kind {
            CastKindRow::CastTime => true,
            CastKindRow::Channeling => {
                config.channel_movement == ChannelMovementPolicy::InterruptOnMove
            }
            CastKindRow::Instant => false,
        };
        let moved = flat_distance(Vec3::from(caster.position), Vec3::from(cast.start_position))
            > MOVEMENT_INTERRUPT_EPSILON;
        if movement_cancels && moved {
            ended.push(EndedCast {
                entity_id: cast.entity_id,
                spell_id: cast.spell_id,
                interrupted: true,
            });
            continue;
        }

        let target_position = cast.target_position.map(Vec3::from);
        let elapsed_seconds = cast.elapsed_seconds + dt;
        let mut channel_tick_accumulator = cast.channel_tick_accumulator;

        let finished = match cast.kind {
            CastKindRow::CastTime => {
                let due = elapsed_seconds >= cast.required_seconds;
                if due {
                    if let Some(cooldown_seconds) = fire_spell(
                        ctx,
                        &caster,
                        spell.as_ref(),
                        target_position,
                        cast.target_entity,
                    ) {
                        start_cooldown(ctx, caster.entity_id, &cast.spell_id, cooldown_seconds);
                    }
                }
                due
            }
            CastKindRow::Channeling => {
                channel_tick_accumulator += dt;
                // A zero interval would spin forever. Bevy had the same hazard
                // and got away with it because `channel_tick_interval_seconds`
                // defaults to 0.25; here the interval is a stored column, so a
                // bad row must not be able to hang the tick.
                let interval = if cast.tick_interval_seconds > 0.0 {
                    cast.tick_interval_seconds
                } else {
                    dt.max(f32::EPSILON)
                };
                while channel_tick_accumulator >= interval {
                    channel_tick_accumulator -= interval;
                    fire_spell(
                        ctx,
                        &caster,
                        spell.as_ref(),
                        target_position,
                        cast.target_entity,
                    );
                }
                cast.required_seconds > 0.0 && elapsed_seconds >= cast.required_seconds
            }
            // Defensive: an instant spell never opens a `cast_state`.
            CastKindRow::Instant => true,
        };

        if finished {
            ended.push(EndedCast {
                entity_id: cast.entity_id,
                spell_id: cast.spell_id,
                interrupted: matches!(cast.kind, CastKindRow::Instant),
            });
        } else {
            ctx.db.cast_state().entity_id().update(CastState {
                elapsed_seconds,
                channel_tick_accumulator,
                ..cast
            });
        }
    }

    for cast in ended {
        end_cast(ctx, cast.entity_id, cast.spell_id, cast.interrupted);
    }
}

/// Bevy's `update_homing_projectiles`, plus the fixed-point case the row schema
/// allows (`target_position` without `target_entity`), which nothing emits yet.
fn update_projectiles(ctx: &ReducerContext, dt: f32) {
    let projectiles: Vec<Projectile> = ctx.db.projectile().iter().collect();

    for mut proj in projectiles {
        proj.remaining_seconds -= dt;
        if proj.remaining_seconds <= 0.0 {
            ctx.db.projectile().id().delete(&proj.id);
            continue;
        }

        let position = Vec3::from(proj.position);
        let destination = match proj.target_entity {
            Some(target) => {
                // A target that died or vanished takes the projectile with it,
                // as it did under Bevy: a homing shot has nothing left to home.
                let Some(entity) = ctx.db.game_entity().entity_id().find(&target) else {
                    ctx.db.projectile().id().delete(&proj.id);
                    continue;
                };
                if !is_alive(ctx, &entity) {
                    ctx.db.projectile().id().delete(&proj.id);
                    continue;
                }
                Vec3::from(entity.position)
            }
            None => match proj.target_position {
                Some(point) => Vec3::from(point),
                None => {
                    log::warn!("projectile {} has no target; removing", proj.id);
                    ctx.db.projectile().id().delete(&proj.id);
                    continue;
                }
            },
        };

        let offset = destination - position;
        let distance = offset.length();
        if distance <= proj.hit_radius {
            match proj.target_entity {
                Some(target) => {
                    crate::sim::combat::apply_damage(ctx, target, Some(proj.caster), proj.damage)
                }
                // A ground-targeted shot has no single victim, so it hits
                // whatever is standing on the impact point — the caster
                // excluded, as for every other area effect in the game.
                None => {
                    for (target, _) in potential_targets(ctx, destination, proj.hit_radius) {
                        if target.get() != proj.caster {
                            crate::sim::combat::apply_damage(
                                ctx,
                                target.get(),
                                Some(proj.caster),
                                proj.damage,
                            );
                        }
                    }
                }
            }
            ctx.db.projectile().id().delete(&proj.id);
            continue;
        }

        let step = (proj.speed * dt).min(distance);
        proj.position = (position + offset / distance * step).into();
        ctx.db.projectile().id().update(proj);
    }
}

/// Bevy's `update_aoe_regions`: tick the wind-up, tick the lifetime, apply to
/// whoever is inside and has not been hit yet, despawn on expiry.
///
/// Generic with respect to the spell, exactly as the original: it reads the
/// payload off the row and never dispatches on `spell_id`.
fn update_aoe_regions(ctx: &ReducerContext, dt: f32) {
    let regions: Vec<AoeRegion> = ctx.db.aoe_region().iter().collect();

    for mut region in regions {
        if region.pending_delay_seconds > 0.0 {
            region.pending_delay_seconds = (region.pending_delay_seconds - dt).max(0.0);
        }
        region.remaining_seconds -= dt;

        // The order matters and is the original's: the delay is ticked first,
        // so a region whose lifetime equals its delay (Meteorite) still gets its
        // one impact on the tick it expires.
        if region.pending_delay_seconds <= 0.0 {
            let shape = match region.shape {
                AoeShapeRow::Circle => AoeShape::Circle,
                AoeShapeRow::Cone => {
                    // Unreachable with the rows this module writes — cones take
                    // the immediate path, because the aperture has no column.
                    log::warn!(
                        "aoe_region {} is a cone with no aperture; skipping its effect",
                        region.id
                    );
                    continue;
                }
            };
            let center = Vec3::from(region.center);
            for (target, position) in potential_targets(ctx, center, region.radius) {
                if region.affected.contains(&target.get()) {
                    continue;
                }
                if !shape.contains(center, region.radius, position) {
                    continue;
                }
                if region.damage > 0.0 {
                    crate::sim::combat::apply_damage(
                        ctx,
                        target.get(),
                        Some(region.caster),
                        region.damage,
                    );
                }
                if region.healing > 0.0 {
                    crate::sim::combat::apply_healing(ctx, target.get(), region.healing);
                }
                region.affected.push(target.get());
            }
        }

        if region.remaining_seconds <= 0.0 {
            ctx.db.aoe_region().id().delete(&region.id);
        } else {
            ctx.db.aoe_region().id().update(region);
        }
    }
}

/// Bevy's `tick_spell_cooldowns` and `tick_ability_cooldowns` in one pass —
/// spells and Eidolon gestures share the `cooldown` table, so they share the
/// tick as well.
///
/// Finished timers are deleted rather than kept at full elapsed, which is what
/// `cleanup_finished` did to the map every tick. The clamp is the one from
/// `spells::components::Cooldown::tick`, restated here because that type hides
/// its fields and cannot be rebuilt from a stored `elapsed`.
fn tick_cooldowns(ctx: &ReducerContext, dt: f32) {
    let cooldowns: Vec<Cooldown> = ctx.db.cooldown().iter().collect();
    for row in cooldowns {
        let elapsed_seconds = (row.elapsed_seconds + dt).min(row.duration_seconds);
        if elapsed_seconds >= row.duration_seconds {
            ctx.db.cooldown().id().delete(&row.id);
        } else {
            ctx.db.cooldown().id().update(Cooldown {
                elapsed_seconds,
                ..row
            });
        }
    }
}

/// The row spelling of a domain [`CastKind`], for the reducers that open a cast.
pub fn cast_kind_row(kind: CastKind) -> CastKindRow {
    match kind {
        CastKind::Instant => CastKindRow::Instant,
        CastKind::CastTime => CastKindRow::CastTime,
        CastKind::Channeling => CastKindRow::Channeling,
    }
}
