//! What a client may ask the spell system to do.
//!
//! The port of Bevy's `process_cast_requests`, `handle_cast_release` and
//! `process_eidolon_cast_requests`. Everything past validation lives in
//! [`crate::sim::spells`]; these three reducers only decide whether the caller
//! is allowed to do what it asked, and open (or close) a `cast_state`.
//!
//! # What changed, and why
//!
//! - **No caster field on the request.** Bevy's `SpellCastRequest` carried the
//!   entity it was cast by, so the handler had to trust the network layer to
//!   have filled it in correctly. `ctx.sender()` is assigned by SpacetimeDB, so
//!   the caster is derived, never claimed.
//! - **Range is enforced.** The Bevy server never checked `cast_range`: the
//!   client decided whether a cast was in range and the server believed it.
//! - **Cancelling a cast reports the cast that was cancelled.** Bevy emitted the
//!   *incoming* spell's id in the `SpellCastEnded` it sent when a new cast
//!   replaced a running one, which made the client hide the wrong bar.
//! - **The boss spellbook path is gone.** These reducers are for players; the
//!   boss casts from `sim::ai` through [`crate::sim::spells::fire_spell`], which
//!   is the same function the hotbar path ends in.

use bevymmo_domain::abilities::{
    cast_inscribed_slot, resolve_active_ability, resolve_slot_preview, AbilityCastMode,
    AbilitySlot, CastBlockedReason, ChannelMovementPolicy as EidolonChannelMovementPolicy,
};
// Legacy spell channeling uses spells::context::ChannelMovementPolicy.
use bevymmo_domain::spells::context::ChannelMovementPolicy as SpellChannelMovementPolicy;
use bevymmo_domain::spells::components::SpellHotbar;
use bevymmo_domain::spells::context::{CastKind, SpellCastContext};
use bevymmo_domain::spells::registry::SpellId;
use bevymmo_domain::EntityId;
use glam::Vec3;
use spacetimedb::{reducer, ReducerContext, Table};

use crate::reducers::lifecycle::caller_entity;
use crate::rows::{equipment_from_rows, known_glyphs_from_rows, Vec3Row};
use crate::sim::spells;
use crate::tables::{
    cast_state, equipment, game_entity, hotbar, known_glyphs, CastKindRow, CastSourceRow, CastState,
    EntityStateRow, GameEntity,
};

/// Casts a spell from the caller's hotbar.
///
/// `target_position` is the aimed ground point; `None` means "wherever I am
/// facing", which is what a self-centred spell wants. `target_entity` is the
/// selected target, required by single-target spells such as Fireball.
///
/// Instant spells resolve inside this call. Cast-time and channelled spells only
/// open a `cast_state`; [`crate::sim::spells::step`] takes it from there.
#[reducer]
pub fn cast_spell(
    ctx: &ReducerContext,
    spell_id: String,
    target_entity: Option<u64>,
    target_position: Option<Vec3Row>,
) -> Result<(), String> {
    let caster = caller_entity(ctx)?;
    if caster.state == EntityStateRow::Dead {
        return Err("dead characters do not cast".to_string());
    }

    let spell = spells::spells()
        .get(&SpellId::new(spell_id.clone()))
        .ok_or_else(|| format!("unknown spell {spell_id:?}"))?;

    // Players cast what is on their hotbar and nothing else. Bevy also allowed
    // a `BossSpellbook` here; the boss no longer goes through a reducer.
    let hotbar: SpellHotbar = ctx
        .db
        .hotbar()
        .identity()
        .find(&ctx.sender())
        .map(|row| (&row.slots).into())
        .unwrap_or_default();
    if !hotbar.contains(&SpellId::new(spell_id.clone())) {
        return Err(format!("{spell_id:?} is not on the hotbar"));
    }

    if spells::is_on_cooldown(ctx, caster.entity_id, &spell_id) {
        return Err(format!("{spell_id:?} is on cooldown"));
    }
    if spells::casting_blocked(ctx, caster.entity_id) {
        return Err("you cannot cast right now".to_string());
    }

    let config = spell.config();
    let target_position = target_position.map(Vec3::from);
    // `cast_range == 0.0` means self-centred: the aimed point is only used for
    // facing, so there is nothing to be out of range of.
    if let Some(target) = target_position {
        let distance = spells::flat_distance(Vec3::from(caster.position), target);
        if config.cast_range > 0.0 && distance > config.cast_range + spells::CAST_RANGE_TOLERANCE {
            return Err(format!(
                "{spell_id:?} reaches {:.1} units, target is {distance:.1} away",
                config.cast_range
            ));
        }
    }

    // Mana is not checked: no spell in `bevymmo_domain` declares a cost, so
    // there is nothing to spend `entity_stats.current_mana` on yet. See the
    // note in the port report.

    let caster = face_target(ctx, caster, target_position);
    cancel_active_cast(ctx, caster.entity_id);

    match spell.cast_kind() {
        CastKind::Instant => {
            if let Some(cooldown_seconds) =
                spells::fire_spell(ctx, &caster, spell.as_ref(), target_position, target_entity)
            {
                spells::start_cooldown(ctx, caster.entity_id, &spell_id, cooldown_seconds);
            }
        }
        kind @ (CastKind::CastTime | CastKind::Channeling) => {
            let channeling = matches!(kind, CastKind::Channeling);
            let tick_interval_seconds = spell.channel_tick_interval_seconds();
            let required_seconds = if channeling {
                config.channel_duration_seconds.unwrap_or(0.0)
            } else {
                config.cast_time_seconds
            };

            // A channel starts armed so its first effect lands on the very next
            // tick: without this a short press looks like the key did nothing.
            let channel_tick_accumulator = if channeling {
                tick_interval_seconds
            } else {
                0.0
            };
            // And its cooldown starts on press, not on release, so holding the
            // key longer cannot also delay the next cast.
            if channeling {
                spells::start_cooldown(ctx, caster.entity_id, &spell_id, config.cooldown_seconds);
            }

            let caster = if matches!(kind, CastKind::CastTime) {
                stop_movement(ctx, caster)
            } else {
                caster
            };
            ctx.db.cast_state().insert(CastState {
                entity_id: caster.entity_id,
                spell_id,
                kind: spells::cast_kind_row(kind),
                source: CastSourceRow::Spell,
                elapsed_seconds: 0.0,
                required_seconds,
                start_position: caster.position,
                target_position: target_position.map(Vec3Row::from),
                target_entity,
                channel_tick_accumulator,
                tick_interval_seconds,
                // Legacy spell: read movement policy from SpellConfig.
                channel_movement_interrupts: matches!(
                    kind,
                    CastKind::Channeling if config.channel_movement == SpellChannelMovementPolicy::InterruptOnMove
                ),
            });
        }
    }

    Ok(())
}

/// Ends the caller's cast of `spell_id`, as on key release.
///
/// A channel that is released has *completed* — its effect has been ticking all
/// along — while a cast-time wind-up released early is a cancellation. Releasing
/// a cast that already ended is not an error: the tick may well have finished it
/// between the key going up and this reducer running.
#[reducer]
pub fn release_cast(ctx: &ReducerContext, spell_id: String) -> Result<(), String> {
    let caster = caller_entity(ctx)?;
    let Some(cast) = ctx.db.cast_state().entity_id().find(&caster.entity_id) else {
        return Ok(());
    };
    if cast.spell_id != spell_id {
        return Ok(());
    }

    let interrupted = !matches!(cast.kind, CastKindRow::Channeling);
    spells::end_cast(ctx, caster.entity_id, cast.spell_id, interrupted);
    Ok(())
}

/// Casts the Eidolon gesture inscribed on the caller's equipped weapon.
///
/// `slot` is `"primary"`, `"secondary"` or `"ultimate"` — the gameplay role, not
/// a keyboard key (see `bevymmo_domain::abilities::AbilitySlot`).
///
/// Branches on the resolved ability's [`AbilityCastMode`]: `Instant` resolves
/// and applies the effect on the spot, `CastTime` and `Channeling` open a
/// `cast_state` row that [`crate::sim::spells::step`] advances, the same way
/// the legacy spell path does.
#[reducer]
pub fn eidolon_cast(
    ctx: &ReducerContext,
    slot: String,
    target_entity: Option<u64>,
    target_position: Option<Vec3Row>,
) -> Result<(), String> {
    let caster = caller_entity(ctx)?;
    if caster.state == EntityStateRow::Dead {
        return Err("dead characters do not cast".to_string());
    }
    let slot = parse_slot(&slot)?;

    let equipment = ctx
        .db
        .equipment()
        .identity()
        .find(&ctx.sender())
        .map(|row| equipment_from_rows(&row.slots))
        .unwrap_or_default();
    let weapon = equipment
        .weapon
        .as_ref()
        .ok_or_else(|| "no weapon equipped".to_string())?;
    let item = spells::items()
        .get(&weapon.item_id)
        .ok_or_else(|| format!("unknown item {:?}", weapon.item_id.as_str()))?;
    let weapon_abilities = item
        .weapon_abilities()
        .ok_or_else(|| format!("{} has no Eidolon gestures", item.display_name()))?;

    let ability_id = resolve_active_ability(slot, weapon_abilities, &weapon.ability_selection)
        .cloned()
        .ok_or_else(|| format!("the weapon offers no gesture for {slot:?}"))?;

    if spells::is_on_cooldown(ctx, caster.entity_id, ability_id.as_str()) {
        return Err(format!("{:?} is on cooldown", ability_id.as_str()));
    }
    if spells::casting_blocked(ctx, caster.entity_id) {
        return Err("you cannot cast right now".to_string());
    }

    let known = ctx
        .db
        .known_glyphs()
        .identity()
        .find(&ctx.sender())
        .map(|row| known_glyphs_from_rows(&row.essences, &row.modifiers, &row.ancient_words))
        .unwrap_or_default();
    let inscriptions = weapon.inscriptions.clone().unwrap_or_default();

    // Resolved first for its `params`, which say how far and how wide the
    // gesture reaches — the radius the target query needs. `cast_inscribed_slot`
    // resolves the same way a line later, which is the point: preview,
    // validation and the cast itself cannot drift apart.
    let preview = resolve_slot_preview(
        slot,
        weapon_abilities,
        &weapon.ability_selection,
        &inscriptions,
        &known,
        spells::base_abilities(),
        spells::modifiers(),
    )
    .map_err(describe_block)?;
    let caster = face_target(ctx, caster, target_position.map(Vec3::from));
    cancel_active_cast(ctx, caster.entity_id);

    // Branch on the resolved ability's cast mode.
    let cast_mode = preview.ability.cast_mode();
    match cast_mode {
        AbilityCastMode::Instant => {
            // Original path: execute immediately.
            let combat = spells::combat_stats(ctx, caster.entity_id)
                .ok_or_else(|| "caster has no stats".to_string())?;
            let target_position = target_position.map(Vec3::from);
            let caster_position = Vec3::from(caster.position);
            let targets = spells::potential_targets(
                ctx,
                caster_position,
                preview.params.range + preview.params.area + spells::TARGET_QUERY_MARGIN,
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
                spells::base_abilities(),
                spells::essences(),
                spells::modifiers(),
                spells::ancient_words(),
                &mut cast_ctx,
            )
            .map_err(describe_block)?;

            spells::apply_pending(
                ctx,
                caster.entity_id,
                caster_position,
                ability_id.as_str(),
                &mut cast_ctx,
            );
            spells::start_cooldown(
                ctx,
                caster.entity_id,
                ability_id.as_str(),
                preview.ability.base_params().cooldown,
            );
            Ok(())
        }
        AbilityCastMode::CastTime => {
            let required_seconds = preview.params.cast_time;
            let target_position = target_position.map(Vec3::from);

            let caster = stop_movement(ctx, caster);
            ctx.db.cast_state().insert(CastState {
                entity_id: caster.entity_id,
                spell_id: ability_id.as_str().to_string(),
                kind: CastKindRow::CastTime,
                source: CastSourceRow::Eidolon,
                elapsed_seconds: 0.0,
                required_seconds,
                start_position: caster.position,
                target_position: target_position.map(Vec3Row::from),
                target_entity,
                channel_tick_accumulator: 0.0,
                tick_interval_seconds: 0.0,
                // CastTime always interrupts on movement; this field is
                // only meaningful for Channeling.
                channel_movement_interrupts: true,
            });
            Ok(())
        }
        AbilityCastMode::Channeling { tick_interval_seconds, movement_policy } => {
            let required_seconds = preview.params.cast_time.max(0.1);
            let target_position = target_position.map(Vec3::from);

            // Channel cooldown starts on press (same as legacy).
            spells::start_cooldown(
                ctx,
                caster.entity_id,
                ability_id.as_str(),
                preview.ability.base_params().cooldown,
            );

            // Store the movement policy from AbilityCastMode so advance_casts
            // can honor it without re-resolving the ability.
            let movement_interrupts = matches!(movement_policy, EidolonChannelMovementPolicy::InterruptOnMove);

            // Channel starts armed so first tick lands on next tick.
            ctx.db.cast_state().insert(CastState {
                entity_id: caster.entity_id,
                spell_id: ability_id.as_str().to_string(),
                kind: CastKindRow::Channeling,
                source: CastSourceRow::Eidolon,
                elapsed_seconds: 0.0,
                required_seconds,
                start_position: caster.position,
                target_position: target_position.map(Vec3Row::from),
                target_entity,
                channel_tick_accumulator: tick_interval_seconds,
                tick_interval_seconds,
                channel_movement_interrupts: movement_interrupts,
            });
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Turns the caster to face the point it aimed at, and returns the updated row.
///
/// Applied only once validation has passed, so a rejected cast cannot silently
/// spin the character around. Self-cast spells send no point and keep the facing
/// they had.
fn face_target(
    ctx: &ReducerContext,
    caster: GameEntity,
    target_position: Option<Vec3>,
) -> GameEntity {
    let Some(target) = target_position else {
        return caster;
    };
    let offset = target - Vec3::from(caster.position);
    let offset = Vec3::new(offset.x, 0.0, offset.z);
    if offset.length() <= 0.001 {
        return caster;
    }
    ctx.db.game_entity().entity_id().update(GameEntity {
        look: offset.normalize().into(),
        ..caster
    })
}

/// Cancels whatever the caster was casting, so starting a spell always replaces
/// the previous one rather than racing it.
fn cancel_active_cast(ctx: &ReducerContext, entity_id: u64) {
    if let Some(active) = ctx.db.cast_state().entity_id().find(&entity_id) {
        spells::end_cast(ctx, entity_id, active.spell_id, true);
    }
}

/// Cast-time spells root the caster for their wind-up rather than allowing a
/// movement command to advance one tick and then cancel the cast.
fn stop_movement(ctx: &ReducerContext, caster: GameEntity) -> GameEntity {
    ctx.db.game_entity().entity_id().update(GameEntity {
        move_target: None,
        state: EntityStateRow::Idle,
        ..caster
    })
}

fn parse_slot(slot: &str) -> Result<AbilitySlot, String> {
    match slot.to_ascii_lowercase().as_str() {
        "primary" => Ok(AbilitySlot::Primary),
        "secondary" => Ok(AbilitySlot::Secondary),
        "ultimate" => Ok(AbilitySlot::Ultimate),
        other => Err(format!(
            "unknown ability slot {other:?}; expected primary, secondary or ultimate"
        )),
    }
}

fn describe_block(reason: CastBlockedReason) -> String {
    match reason {
        CastBlockedReason::UnknownGlyph => {
            "you do not know every glyph inscribed on that slot".to_string()
        }
        CastBlockedReason::MissingRegistryEntry => {
            "that gesture no longer exists in the registry".to_string()
        }
    }
}
