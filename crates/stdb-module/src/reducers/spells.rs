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
    cast_armor_inscribed_ability, cast_inscribed_slot, cast_root_inscribed_slot,
    resolve_active_ability, resolve_armor_inscribed_ability, resolve_root_inscribed_slot,
    resolve_slot_preview, AbilityCastMode,
    AbilitySlot, BlueprintExecution, CastBlockedReason, ChannelMovementPolicy as EidolonChannelMovementPolicy,
};
// Legacy spell channeling uses spells::context::ChannelMovementPolicy.
use bevymmo_domain::spells::context::ChannelMovementPolicy as SpellChannelMovementPolicy;
use bevymmo_domain::items::components::EquipSlot;
use bevymmo_domain::spells::components::SpellHotbar;
use bevymmo_domain::spells::context::{CastKind, SpellCastContext};
use bevymmo_domain::spells::registry::SpellId;
use bevymmo_domain::EntityId;
use glam::Vec3;
use spacetimedb::{reducer, ReducerContext, Table};

use crate::reducers::lifecycle::caller_entity;
use crate::rows::{
    equipment_from_rows, known_ancient_language_from_rows, known_glyphs_from_rows, Vec3Row,
};
use crate::sim::spells::{self, ability_loadout_for_item, fire_eidolon_ability};
use crate::tables::{
    cast_state, equipment, game_entity, hotbar, known_ancient_language, known_glyphs, CastKindRow,
    CastSourceRow, CastState, EntityStateRow, GameEntity,
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

    match cast.kind {
        CastKindRow::Channeling => {
            // Channeling ends without interruption (ran full duration or player released).
            spells::end_cast(ctx, caster.entity_id, cast.spell_id, false);
        }
        CastKindRow::Charge => {
            // Charge fires on release. Resolve and fire the ability now.
            if let Some(caster_entity) = ctx.db.game_entity().entity_id().find(&caster.entity_id) {
                let target_position = cast.target_position.map(Vec3::from);
                match fire_eidolon_ability(
                    ctx,
                    &caster_entity,
                    &cast.spell_id,
                    target_position,
                    cast.target_entity,
                    cast.source,
                ) {
                    Some(cd) => {
                        spells::start_cooldown(ctx, caster.entity_id, &cast.spell_id, cd);
                        spells::end_cast(ctx, caster.entity_id, cast.spell_id, false);
                    }
                    None => {
                        // Resolution failed (equipment changed, etc.) — interrupt.
                        spells::end_cast(ctx, caster.entity_id, cast.spell_id, true);
                    }
                }
            } else {
                spells::end_cast(ctx, caster.entity_id, cast.spell_id, true);
            }
        }
        // CastTime or Instant should not normally receive release_cast (Instant
        // doesn't open a cast_state; CastTime auto-fires in advance_casts). Treat
        // as interruption for safety.
        _ => {
            spells::end_cast(ctx, caster.entity_id, cast.spell_id, true);
        }
    }
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
    let weapon_abilities = ability_loadout_for_item(item.as_ref())
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

    // RootWord inscriptions use the new knowledge and blueprint pipeline;
    // legacy instances keep the old path during migration.
    let preview = if let Some(root_inscription) = weapon.root_inscription.as_ref() {
        let known_language = ctx
            .db
            .known_ancient_language()
            .identity()
            .find(&ctx.sender())
            .map(|row| {
                known_ancient_language_from_rows(
                    &row.root_words,
                    &row.ancient_words,
                    &row.base_abilities,
                )
            })
            .ok_or_else(|| "ancient language has not been initialized".to_string())?;
        resolve_root_inscribed_slot(
            slot,
            weapon_abilities,
            &weapon.ability_selection,
            root_inscription,
            &known_language,
            spells::base_abilities(),
            spells::root_words(),
            spells::ancient_words(),
            Some(item.as_ref()),
        )
    } else {
        resolve_slot_preview(
            slot,
            weapon_abilities,
            &weapon.ability_selection,
            &inscriptions,
            &known,
            spells::base_abilities(),
            spells::modifiers(),
            Some(item.as_ref()),
        )
    }
    .map_err(describe_block)?;
    let caster = face_target(ctx, caster, target_position.map(Vec3::from));
    cancel_active_cast(ctx, caster.entity_id);

    // Branch on the resolved ability's cast mode and execution.
    let cast_mode = preview.ability.cast_mode();
    let is_charge = preview.blueprint.execution == BlueprintExecution::Charge;

    match (cast_mode, is_charge) {
        (AbilityCastMode::Instant, _) => {
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

            if let Some(root_inscription) = weapon.root_inscription.as_ref() {
                let known_language = ctx
                    .db
                    .known_ancient_language()
                    .identity()
                    .find(&ctx.sender())
                    .map(|row| {
                        known_ancient_language_from_rows(
                            &row.root_words,
                            &row.ancient_words,
                            &row.base_abilities,
                        )
                    })
                    .ok_or_else(|| "ancient language has not been initialized".to_string())?;
                cast_root_inscribed_slot(
                    slot,
                    weapon_abilities,
                    &weapon.ability_selection,
                    root_inscription,
                    &known_language,
                    spells::base_abilities(),
                    spells::root_words(),
                    spells::ancient_words(),
                    &mut cast_ctx,
                    Some(item.as_ref()),
                )
            } else {
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
                    Some(item.as_ref()),
                )
            }
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
        (AbilityCastMode::CastTime, false) => {
            // Standard CastTime wind-up (auto-fires when elapsed >= required).
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
        (AbilityCastMode::CastTime, true) => {
            // Charge execution: hold-to-charge, fires on release (not auto-fire).
            // required_seconds is the max charge duration; the ability scales with
            // how long the player held before releasing.
            let required_seconds = preview.params.cast_time;
            let target_position = target_position.map(Vec3::from);

            let caster = stop_movement(ctx, caster);
            ctx.db.cast_state().insert(CastState {
                entity_id: caster.entity_id,
                spell_id: ability_id.as_str().to_string(),
                kind: CastKindRow::Charge,
                source: CastSourceRow::Eidolon,
                elapsed_seconds: 0.0,
                required_seconds,
                start_position: caster.position,
                target_position: target_position.map(Vec3Row::from),
                target_entity,
                channel_tick_accumulator: 0.0,
                tick_interval_seconds: 0.0,
                // Charge interrupts on movement (same as CastTime).
                channel_movement_interrupts: true,
            });
            Ok(())
        }
        (AbilityCastMode::Channeling { tick_interval_seconds, movement_policy }, _) => {
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

/// Casts the first Primary ability supplied by an equipped armor item.
///
/// Armor abilities intentionally use a separate API from weapon Eidolon slots.
/// This initial reducer handles instant armor abilities; timed armor casts will
/// reuse the same source-aware resolver in the scheduler.
#[reducer]
pub fn armor_cast(
    ctx: &ReducerContext,
    armor_slot: String,
    target_entity: Option<u64>,
    target_position: Option<Vec3Row>,
) -> Result<(), String> {
    let caster = caller_entity(ctx)?;
    if caster.state == EntityStateRow::Dead {
        return Err("dead characters do not cast".to_string());
    }
    let target_slot = match armor_slot.to_ascii_lowercase().as_str() {
        "helmet" => EquipSlot::Helmet,
        "armor" | "chest" | "chestplate" => EquipSlot::Armor,
        "shoes" | "boots" => EquipSlot::Shoes,
        other => return Err(format!("unknown armor slot {other:?}")),
    };
    let equipment = ctx
        .db
        .equipment()
        .identity()
        .find(&ctx.sender())
        .map(|row| equipment_from_rows(&row.slots))
        .unwrap_or_default();
    let armor = equipment
        .get(target_slot)
        .as_ref()
        .ok_or_else(|| format!("armor slot {armor_slot:?} is empty"))?;
    let item = spells::items()
        .get(&armor.item_id)
        .ok_or_else(|| format!("unknown item {:?}", armor.item_id.as_str()))?;
    let abilities = ability_loadout_for_item(item.as_ref())
        .ok_or_else(|| format!("{} has no armor abilities", item.display_name()))?;
    let ability_id = abilities
        .primary
        .first()
        .cloned()
        .ok_or_else(|| "armor has no Primary ability".to_string())?;
    if spells::is_on_cooldown(ctx, caster.entity_id, ability_id.as_str()) {
        return Err(format!("{:?} is on cooldown", ability_id.as_str()));
    }
    if spells::casting_blocked(ctx, caster.entity_id) {
        return Err("you cannot cast right now".to_string());
    }

    let language_row = ctx
        .db
        .known_ancient_language()
        .identity()
        .find(&ctx.sender())
        .ok_or_else(|| "ancient language has not been initialized".to_string())?;
    let language = known_ancient_language_from_rows(
        &language_row.root_words,
        &language_row.ancient_words,
        &language_row.base_abilities,
    );
    let preview = resolve_armor_inscribed_ability(
        &ability_id,
        armor.armor_inscription.as_ref(),
        &language,
        spells::base_abilities(),
        spells::root_words(),
        spells::ancient_words(),
        Some(item.as_ref()),
    )
    .map_err(describe_block)?;
    let cast_mode = preview.ability.cast_mode();
    let is_charge = preview.blueprint.execution == BlueprintExecution::Charge;
    let source = match target_slot {
        EquipSlot::Helmet => CastSourceRow::Helmet,
        EquipSlot::Armor => CastSourceRow::Armor,
        EquipSlot::Shoes => CastSourceRow::Shoes,
        _ => return Err("invalid armor source".to_string()),
    };

    let caster = face_target(ctx, caster, target_position.map(Vec3::from));
    cancel_active_cast(ctx, caster.entity_id);
    if matches!(cast_mode, AbilityCastMode::Instant) {
        return cast_armor_instant(
            ctx, caster, target_position, target_entity, &ability_id, &preview,
            armor, &language, item.as_ref(),
        );
    }

    let target_position = target_position.map(Vec3::from);
    let caster = stop_movement(ctx, caster);
    let (kind, required_seconds, tick_interval_seconds, channel_movement_interrupts) = match cast_mode {
        AbilityCastMode::CastTime if is_charge => (CastKindRow::Charge, preview.params.cast_time, 0.0, true),
        AbilityCastMode::CastTime => (CastKindRow::CastTime, preview.params.cast_time, 0.0, true),
        AbilityCastMode::Channeling { tick_interval_seconds, movement_policy } => (
            CastKindRow::Channeling,
            preview.params.cast_time.max(0.1),
            tick_interval_seconds,
            matches!(movement_policy, EidolonChannelMovementPolicy::InterruptOnMove),
        ),
        AbilityCastMode::Instant => unreachable!(),
    };
    if matches!(kind, CastKindRow::Channeling) {
        spells::start_cooldown(ctx, caster.entity_id, ability_id.as_str(), preview.ability.base_params().cooldown);
    }
    ctx.db.cast_state().insert(CastState {
        entity_id: caster.entity_id,
        spell_id: ability_id.as_str().to_string(),
        kind,
        source,
        elapsed_seconds: 0.0,
        required_seconds,
        start_position: caster.position,
        target_position: target_position.map(Vec3Row::from),
        target_entity,
        channel_tick_accumulator: if matches!(kind, CastKindRow::Channeling) { tick_interval_seconds } else { 0.0 },
        tick_interval_seconds,
        channel_movement_interrupts,
    });
    Ok(())
}

fn cast_armor_instant(
    ctx: &ReducerContext,
    caster: crate::tables::GameEntity,
    target_position: Option<Vec3Row>,
    target_entity: Option<u64>,
    ability_id: &bevymmo_domain::abilities::AbilityId,
    preview: &bevymmo_domain::abilities::SlotPreview,
    armor: &bevymmo_domain::items::instance::ItemInstance,
    language: &bevymmo_domain::abilities::KnownAncientLanguage,
    item: &dyn bevymmo_domain::items::definition::Item,
) -> Result<(), String> {
    let combat = spells::combat_stats(ctx, caster.entity_id)
        .ok_or_else(|| "caster has no stats".to_string())?;
    let caster_position = Vec3::from(caster.position);
    let targets = spells::potential_targets(ctx, caster_position, preview.params.range + preview.params.area + spells::TARGET_QUERY_MARGIN);
    let mut cast_ctx = SpellCastContext::new(
        EntityId::new(caster.entity_id), caster_position, &combat,
        Vec3::from(caster.look), target_position.map(Vec3::from),
        target_entity.map(EntityId::new), &targets,
    );
    cast_armor_inscribed_ability(
        ability_id, armor.armor_inscription.as_ref(), language,
        spells::base_abilities(), spells::root_words(), spells::ancient_words(),
        &mut cast_ctx, Some(item),
    ).map_err(describe_block)?;
    spells::apply_pending(ctx, caster.entity_id, caster_position, ability_id.as_str(), &mut cast_ctx);
    spells::start_cooldown(ctx, caster.entity_id, ability_id.as_str(), preview.ability.base_params().cooldown);
    Ok(())
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
        CastBlockedReason::UnknownRootWord => {
            "that slot uses an unknown or unavailable Root Word".to_string()
        }
        CastBlockedReason::UnknownAncientWord => {
            "that slot uses an unknown or unavailable Ancient Word".to_string()
        }
        CastBlockedReason::IncompatibleAncientWord => {
            "an Ancient Word is incompatible with the selected gesture".to_string()
        }
    }
}
