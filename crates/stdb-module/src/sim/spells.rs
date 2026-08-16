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

use bevymmo_domain::effects::{
    ApplyStatusEffect, CleanseEffect, DamageEffect, EffectBundle, EffectContext, EffectSpec,
    HealEffect, PurgeEffect, StatusFilter, StatusId, StatusSelection,
};
use bevymmo_domain::items::registry::ItemRegistry;
use bevymmo_domain::spells::components::MOVEMENT_INTERRUPT_EPSILON;
use bevymmo_domain::spells::context::{
    AoeShape, AoeSpawnRequest, AoeTargeting, CastKind, ProjectileSpawnRequest, Spell,
    SpellCastContext,
};
use bevymmo_domain::spells::registry::{SpellId, SpellRegistry};
use bevymmo_domain::stats::components::{CombatStats, StatsBundleData};
use bevymmo_domain::stats::events::{
    ApplyStatModifierEvent, ModifierEffect, ModifierKind, ModifierOp,
};
use bevymmo_domain::EntityId;
use glam::Vec3;
use spacetimedb::{ReducerContext, Table};

use crate::rows::{
    EffectPayloadFilterRow, EffectPayloadKindRow, EffectPayloadRow, EffectPayloadSelectionRow,
    Vec3Row,
};
use crate::tables::{
    aoe_region, cast_ended, cast_state, cooldown, entity_stats, game_entity,
    grid_cell, projectile, spell_visual_effect, AoeRegion, AoeShapeRow, AoeTargetingRow,
    CastEndedEvent, CastKindRow, CastSourceRow, CastState, Cooldown, EntityStateRow,
    GameEntity, ModifierKindRow, Projectile, SpellVisualEffectEvent,
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
    REGISTRY.get_or_init(bevymmo_domain::content::spells::default_spells)
}

pub fn base_abilities() -> &'static BaseAbilityRegistry {
    static REGISTRY: OnceLock<BaseAbilityRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::abilities::default_base_abilities)
}

pub fn essences() -> &'static EssenceRegistry {
    static REGISTRY: OnceLock<EssenceRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::essences::default_essences)
}

pub fn modifiers() -> &'static ModifierRegistry {
    static REGISTRY: OnceLock<ModifierRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::modifiers::default_modifiers)
}

pub fn ancient_words() -> &'static AncientWordRegistry {
    static REGISTRY: OnceLock<AncientWordRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::ancient_words::default_ancient_words)
}

/// The item catalogue, needed to read the equipped weapon's Eidolon gestures.
///
/// Lives here rather than in `reducers::items` because the spell path is its
/// only consumer today; move it if the inventory reducers grow one.
pub fn items() -> &'static ItemRegistry {
    static REGISTRY: OnceLock<ItemRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::items::default_items)
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

/// Fires an Eidolon ability by re-resolving equipment and inscriptions.
///
/// Used by `advance_casts` when a CastTime/Channeling Eidolon cast completes.
/// Returns the base cooldown duration, or `None` if the caster lost their
/// weapon/stats between starting and finishing the cast.
pub fn fire_eidolon_ability(
    ctx: &ReducerContext,
    caster: &GameEntity,
    ability_id_str: &str,
    target_position: Option<Vec3>,
    target_entity: Option<u64>,
) -> Option<f32> {
    use bevymmo_domain::abilities::{
        cast_inscribed_slot, resolve_active_ability, AbilitySlot,
    };
    use crate::rows::{equipment_from_rows, known_glyphs_from_rows};
    use crate::tables::{equipment, known_glyphs};

    let combat = combat_stats(ctx, caster.entity_id)?;
    let caster_position = Vec3::from(caster.position);

    // `ctx.sender()` is the *module's* identity here: this runs from
    // `advance_casts`, inside the scheduled `game_tick` reducer, not from a
    // call the player made directly. The caster's own identity — who started
    // the cast — is `caster.owner`, not the sender of whatever reducer
    // happens to be running this tick. Using `ctx.sender()` made every
    // CastTime/Channeling Eidolon ability resolve against an identity with no
    // rows at all, so every one of them silently failed to fire.
    let identity = caster.owner?;

    // Re-resolve equipment and weapon (must still be equipped).
    let equip_row = ctx.db.equipment().identity().find(&identity)?;
    let equipment = equipment_from_rows(&equip_row.slots);
    let weapon = equipment.weapon.as_ref()?;
    let item = items().get(&weapon.item_id)?;
    let weapon_abilities = item.ability_loadout()?;

    // Determine which slot this ability belongs to.
    let ability_id = bevymmo_domain::abilities::AbilityId::new(ability_id_str.to_string());
    let slot = [AbilitySlot::Primary, AbilitySlot::Secondary, AbilitySlot::Ultimate]
        .into_iter()
        .find(|&s| {
            resolve_active_ability(s, weapon_abilities, &weapon.ability_selection)
                .map_or(false, |id| id.as_str() == ability_id.as_str())
        })?;

    // Resolve inscriptions and known glyphs.
    let known_row = ctx.db.known_glyphs().identity().find(&identity);
    let known = known_row
        .map(|r| known_glyphs_from_rows(&r.essences, &r.modifiers, &r.ancient_words))
        .unwrap_or_default();
    let inscriptions = weapon.inscriptions.clone().unwrap_or_default();

    // Build target list.
    let preview = {
        use bevymmo_domain::abilities::resolve_slot_preview;
        resolve_slot_preview(
            slot,
            weapon_abilities,
            &weapon.ability_selection,
            &inscriptions,
            &known,
            base_abilities(),
            modifiers(),
            Some(item.as_ref()),
        )
        .ok()?
    };

    let targets = potential_targets(
        ctx,
        caster_position,
        preview.params.range + preview.params.area + TARGET_QUERY_MARGIN,
    );

    let mut cast_ctx = SpellCastContext::new(
        EntityId::new(caster.entity_id),
        caster_position,
        &combat,
        Vec3::from(caster.look),
        target_position,
        target_entity.map(EntityId::new),
        &targets,
    );

    cast_inscribed_slot(
        slot,
        weapon_abilities,
        &weapon.ability_selection,
        &inscriptions,
        &known,
        base_abilities(),
        essences(),
        modifiers(),
        ancient_words(),
        &mut cast_ctx,
        Some(item.as_ref()),
    )
    .ok()?;

    apply_pending(ctx, caster.entity_id, caster_position, ability_id_str, &mut cast_ctx);
    Some(preview.ability.base_params().cooldown)
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
    for bundle in cast.pending_effects.drain(..) {
        crate::sim::effects::resolve_bundle(ctx, 0, bundle);
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
        effects: request.effects.iter().map(EffectPayloadRow::from).collect(),
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
                    event.source.map(|s| s.get()),
                    &format!("{field:?}"),
                    *value,
                    is_multiplicative,
                    modifier_row_kind(event.kind),
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


/// Carries the caster's own buff/debuff label into the row.
///
/// Inferring it from the sign would get `-0.3 Armor` right and a reduced
/// incoming-damage modifier wrong, so the declared value is the one stored.
fn modifier_row_kind(kind: ModifierKind) -> ModifierKindRow {
    match kind {
        ModifierKind::Buff => ModifierKindRow::Buff,
        ModifierKind::Debuff => ModifierKindRow::Debuff,
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
    if request.effects.is_empty() {
        // No effects to persist — take the immediate path.
        return None;
    }
    // `affected` seeds the caster for ExcludeCaster so the row alone enforces
    // the policy without a separate targeting column read at tick time.
    let affected = match request.targeting {
        AoeTargeting::Everyone => Vec::new(),
        AoeTargeting::ExcludeCaster => vec![caster],
        AoeTargeting::CasterOnly => vec![caster],
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
        targeting: targeting_row(request.targeting),
        effects: request.effects.iter().map(EffectPayloadRow::from).collect(),
    })
}

/// Applies an AoE request immediately to everything currently inside it.
fn apply_aoe_now(ctx: &ReducerContext, caster: u64, request: &AoeSpawnRequest) {
    let targeting = request.targeting;
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
        let payloads: Vec<_> = request
            .effects
            .iter()
            .map(EffectPayloadRow::from)
            .collect();
        if !payloads.is_empty() {
            resolve_payloads(ctx, &payloads, target.get(), Some(caster));
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
///
/// Handles both legacy [`CastSourceRow::Spell`] and [`CastSourceRow::Eidolon`] casts.
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

        // --- Movement interrupt check (source-agnostic) ---
        // CastTime always interrupts on movement.
        // Channeling respects the stored channel_movement_interrupts policy,
        // which was captured from SpellConfig (legacy) or AbilityCastMode (Eidolon)
        // at cast start time.
        let movement_cancels = match cast.kind {
            CastKindRow::CastTime | CastKindRow::Charge => true,
            CastKindRow::Channeling => cast.channel_movement_interrupts,
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
        let mut eidolon_cast_failed = false; // Tracks resolution failure for CastTime

        let finished = match (cast.source, cast.kind) {
            // --- Legacy Spell paths (unchanged behaviour) ---
            (CastSourceRow::Spell, CastKindRow::CastTime) => {
                let Some(spell) = spells().get(&SpellId::new(cast.spell_id.clone())) else {
                    log::warn!("cast in progress for unknown spell {:?}; cancelling", cast.spell_id);
                    ended.push(EndedCast { entity_id: cast.entity_id, spell_id: cast.spell_id, interrupted: true });
                    continue;
                };
                let due = elapsed_seconds >= cast.required_seconds;
                if due {
                    if let Some(cd) = fire_spell(ctx, &caster, spell.as_ref(), target_position, cast.target_entity) {
                        start_cooldown(ctx, caster.entity_id, &cast.spell_id, cd);
                    }
                }
                due
            }
            (CastSourceRow::Spell, CastKindRow::Charge) => {
                // Charge is an Eidolon-only execution. A legacy spell carrying
                // this state is invalid, so close it without firing.
                log::warn!("legacy spell {:?} entered charge state; cancelling", cast.spell_id);
                true
            }
            (CastSourceRow::Spell, CastKindRow::Channeling) => {
                let Some(spell) = spells().get(&SpellId::new(cast.spell_id.clone())) else {
                    log::warn!("cast in progress for unknown spell {:?}; cancelling", cast.spell_id);
                    ended.push(EndedCast { entity_id: cast.entity_id, spell_id: cast.spell_id, interrupted: true });
                    continue;
                };
                channel_tick_accumulator += dt;
                let interval = if cast.tick_interval_seconds > 0.0 { cast.tick_interval_seconds } else { dt.max(f32::EPSILON) };
                while channel_tick_accumulator >= interval {
                    channel_tick_accumulator -= interval;
                    fire_spell(ctx, &caster, spell.as_ref(), target_position, cast.target_entity);
                }
                cast.required_seconds > 0.0 && elapsed_seconds >= cast.required_seconds
            }

            // --- Eidolon ability paths ---
            (CastSourceRow::Eidolon, CastKindRow::CastTime) => {
                let due = elapsed_seconds >= cast.required_seconds;
                if due {
                    // Resolution may fail if equipment/selection changed during wind-up.
                    // Treat as interrupted: no effect, no cooldown (client shows cancelled bar).
                    match fire_eidolon_ability(
                        ctx, &caster, &cast.spell_id, target_position, cast.target_entity,
                    ) {
                        Some(cd) => {
                            start_cooldown(ctx, caster.entity_id, &cast.spell_id, cd);
                            true // Completed successfully
                        }
                        None => {
                            // Equipment changed or weapon removed during cast.
                            log::info!(
                                "Eidolon cast {:?} for entity {} failed at completion; interrupting",
                                cast.spell_id, cast.entity_id
                            );
                            eidolon_cast_failed = true;
                            true // End the cast (will be marked as interrupted below)
                        }
                    }
                } else {
                    false
                }
            }
            (CastSourceRow::Eidolon, CastKindRow::Channeling) => {
                channel_tick_accumulator += dt;
                let interval = if cast.tick_interval_seconds > 0.0 { cast.tick_interval_seconds } else { dt.max(f32::EPSILON) };
                while channel_tick_accumulator >= interval {
                    channel_tick_accumulator -= interval;
                    // Re-fire each tick (same as legacy channeling).
                    // Tick failures are logged but don't interrupt the channel:
                    // the player may have moved out of range or the target died,
                    // but the channel itself is still valid.
                    if fire_eidolon_ability(ctx, &caster, &cast.spell_id, target_position, cast.target_entity).is_none() {
                        log::debug!(
                            "Eidolon channel tick {:?} for entity {} failed to resolve",
                            cast.spell_id, cast.entity_id
                        );
                    }
                }
                cast.required_seconds > 0.0 && elapsed_seconds >= cast.required_seconds
            }
            (CastSourceRow::Eidolon, CastKindRow::Charge) => {
                // Charge accumulates while held but does NOT auto-fire.
                // The ability fires when the player releases (release_cast reducer).
                // If elapsed exceeds required_seconds, the charge is "full" but
                // we still wait for release.
                false
            }

            // Defensive: an instant spell/ability never opens a `cast_state`.
            (_, CastKindRow::Instant) => true,
        };

        if finished {
            // Determine if this was a true completion or an interruption.
            // Instant casts are never interruptions. Eidolon CastTime that failed
            // resolution is interrupted. Everything else depends on kind.
            let interrupted = matches!(cast.kind, CastKindRow::Instant)
                || eidolon_cast_failed;

            ended.push(EndedCast {
                entity_id: cast.entity_id,
                spell_id: cast.spell_id,
                interrupted,
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

fn resolve_payloads(
    ctx: &ReducerContext,
    payloads: &[EffectPayloadRow],
    target: u64,
    source: Option<u64>,
) {
    let effects = payloads
        .iter()
        .filter_map(|payload| match payload.kind {
            EffectPayloadKindRow::Damage => {
                Some(EffectSpec::Damage(DamageEffect { amount: payload.amount }))
            }
            EffectPayloadKindRow::Heal => {
                Some(EffectSpec::Heal(HealEffect { amount: payload.amount }))
            }
            EffectPayloadKindRow::ApplyStatus => {
                let status_id = payload.status_id.as_deref()?.to_string();
                Some(EffectSpec::ApplyStatus(ApplyStatusEffect {
                    status_id: StatusId::new(status_id),
                    duration_override_seconds: payload.duration_override_seconds,
                    potency: payload.potency,
                }))
            }
            EffectPayloadKindRow::Cleanse => Some(EffectSpec::Cleanse(CleanseEffect {
                filter: payload.status_filter.map(status_filter).unwrap_or(StatusFilter::All),
                max_statuses: payload.max_statuses,
                selection: payload
                    .selection
                    .map(status_selection)
                    .unwrap_or(StatusSelection::Oldest),
            })),
            EffectPayloadKindRow::Purge => Some(EffectSpec::Purge(PurgeEffect {
                filter: payload.status_filter.map(status_filter).unwrap_or(StatusFilter::All),
                max_statuses: payload.max_statuses,
                selection: payload
                    .selection
                    .map(status_selection)
                    .unwrap_or(StatusSelection::Oldest),
            })),
        })
        .collect();

    let mut context = EffectContext::new(EntityId::new(target));
    context.source = source.map(EntityId::new);
    crate::sim::effects::resolve_bundle(ctx, 0, EffectBundle::new(context, effects));
}

fn status_filter(filter: EffectPayloadFilterRow) -> StatusFilter {
    match filter {
        EffectPayloadFilterRow::Buffs => StatusFilter::Buffs,
        EffectPayloadFilterRow::Debuffs => StatusFilter::Debuffs,
        EffectPayloadFilterRow::All => StatusFilter::All,
    }
}

fn status_selection(selection: EffectPayloadSelectionRow) -> StatusSelection {
    match selection {
        EffectPayloadSelectionRow::Oldest => StatusSelection::Oldest,
        EffectPayloadSelectionRow::Newest => StatusSelection::Newest,
        EffectPayloadSelectionRow::ShortestRemaining => StatusSelection::ShortestRemaining,
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
                    resolve_payloads(ctx, &proj.effects, target, Some(proj.caster));
                }
                // A ground-targeted shot has no single victim, so it hits
                // whatever is standing on the impact point — the caster
                // excluded, as for every other area effect in the game.
                None => {
                    for (target, _) in potential_targets(ctx, destination, proj.hit_radius) {
                        if target.get() != proj.caster {
                            resolve_payloads(
                                ctx,
                                &proj.effects,
                                target.get(),
                                Some(proj.caster),
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
            let targeting = targeting_from_row(region.targeting);
            let caster_id = EntityId::new(region.caster);
            for (target, position) in potential_targets(ctx, center, region.radius) {
                if region.affected.contains(&target.get()) {
                    continue;
                }
                if !shape.contains(center, region.radius, position) {
                    continue;
                }
                if !targeting.allows(caster_id, target) {
                    continue;
                }
                resolve_payloads(ctx, &region.effects, target.get(), Some(region.caster));
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

/// Domain [`AoeTargeting`] → row [`AoeTargetingRow`].
fn targeting_row(targeting: AoeTargeting) -> AoeTargetingRow {
    match targeting {
        AoeTargeting::Everyone => AoeTargetingRow::Everyone,
        AoeTargeting::CasterOnly => AoeTargetingRow::CasterOnly,
        AoeTargeting::ExcludeCaster => AoeTargetingRow::ExcludeCaster,
    }
}

/// Row [`AoeTargetingRow`] → domain [`AoeTargeting`].
fn targeting_from_row(row: AoeTargetingRow) -> AoeTargeting {
    match row {
        AoeTargetingRow::Everyone => AoeTargeting::Everyone,
        AoeTargetingRow::CasterOnly => AoeTargeting::CasterOnly,
        AoeTargetingRow::ExcludeCaster => AoeTargeting::ExcludeCaster,
    }
}
