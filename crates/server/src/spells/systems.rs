//! Server-authoritative systems for the spell cast pipeline.
//!
//! Three timing models are supported:
//! - [`CastKind::Instant`]: immediate effect on key press.
//! - [`CastKind::CastTime`]: blocking wind-up, movement always cancels.
//! - [`CastKind::Channeling`]: tick-repeated effect while held down,
//!   per-spell configurable movement policy.

use bevy::prelude::*;

use bevymmo_shared::entity::boss::components::BossSpellbook;
use bevymmo_shared::entity::components::GameEntity;
use bevymmo_shared::entity::player::components::Player;
use bevymmo_shared::network::protocol::{
    Channel1, Inputs, LookDirection, NetworkEntityId, Position, SpellCastEnded, SpellCastProgress,
    SpellVisualEffect,
};
use bevymmo_shared::spells::components::{
    CastProgress, SpellCooldowns, SpellHotbar, MOVEMENT_INTERRUPT_EPSILON,
};
use bevymmo_shared::spells::context::{CastKind, SpellCastContext};
use bevymmo_shared::spells::events::{SpellCastRequest, SpellReleaseRequest};
use bevymmo_shared::spells::registry::SpellRegistry;
use bevymmo_shared::spells::{ChannelMovementPolicy, SpellId};
use bevymmo_shared::stats::components::{CombatStats, VitalStats};
use bevymmo_shared::stats::events::{ApplyStatModifierEvent, DamageEvent, HealEvent};
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::{NetworkTarget, ServerMultiMessageSender};

use crate::spells::{aoe::spawn_aoe_region, projectile::spawn_fireball_projectile};

const CAST_PROGRESS_REPLICATION_INTERVAL_SECONDS: f32 = 0.1;

/// Processes cast requests from clients.
///
/// Instant spells execute immediately (historical behavior).
/// CastTime/Channeling spells do not run `cast` here: they spawn a
/// [`CastProgress`] component on the caster, which system [`advance_cast_progress`]
/// ticks until completion. Before spawning a new `CastProgress`,
/// any active cast on the caster is cancelled with a emitted [`SpellCastEnded`].
#[allow(clippy::type_complexity)]
pub fn process_cast_requests(
    mut commands: Commands,
    mut requests: MessageReader<SpellCastRequest>,
    registry: Res<SpellRegistry>,
    mut casters: Query<(
        Option<&SpellHotbar>,
        &mut SpellCooldowns,
        &Position,
        &mut LookDirection,
        Option<&CastProgress>,
        Option<&bevymmo_shared::crowd_control::CrowdControlState>,
    )>,
    caster_stats: Query<&CombatStats>,
    caster_inputs: Query<&ActionState<Inputs>>,
    caster_network_ids: Query<&NetworkEntityId>,
    boss_spellbooks: Query<&BossSpellbook>,
    targets_query: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut heal_events: MessageWriter<HealEvent>,
    mut stat_modifier_events: MessageWriter<ApplyStatModifierEvent>,
    mut visual_sender: ServerMultiMessageSender,
    mut local_visuals: MessageWriter<SpellVisualEffect>,
    mut local_cast_ended: MessageWriter<SpellCastEnded>,
    server: Single<&lightyear::prelude::server::Server>,
) {
    let server = server.into_inner();

    for request in requests.read() {
        // Step 1: lookup spell
        let Some(spell) = registry.get(&request.spell_id) else {
            bevy::log::warn!("Unknown spell cast request: {:?}", request.spell_id);
            continue;
        };
        let spell_config = spell.config();

        // Step 2: validate hotbar + cooldowns
        let Ok((hotbar, cooldowns, caster_position, mut look_direction, existing_cast, cc_state)) =
            casters.get_mut(request.caster)
        else {
            // A request can refer to an entity that was despawned between
            // emission and processing. Do not emit a warning every fixed tick.
            bevy::log::debug!(
                "Ignoring spell request from missing caster {}",
                request.caster
            );
            continue;
        };

        // Players cast via hotbar; the boss casts via its BossSpellbook
        // (it has more than 3 abilities, so it bypasses the Q/W/E hotbar).
        let in_boss_spellbook = boss_spellbooks
            .get(request.caster)
            .is_ok_and(|spellbook| spellbook.contains(&request.spell_id));
        if !in_boss_spellbook && !hotbar.is_some_and(|hotbar| hotbar.contains(&request.spell_id)) {
            bevy::log::warn!("Caster attempted to cast a spell not assigned to the hotbar");
            continue;
        }

        if cooldowns.is_on_cooldown(&request.spell_id) {
            bevy::log::debug!(
                "Spell {:?} on cooldown for caster {}",
                request.spell_id,
                request.caster
            );
            continue;
        }

        if cc_state.map(|c| c.has_blocking_cc()).unwrap_or(false) {
            bevy::log::debug!(
                "Caster {} is CC'd, rejecting spell {:?}",
                request.caster,
                request.spell_id
            );
            continue;
        }

        // Drop the borrow before potentially spawning CastProgress / firing.
        let caster_position = caster_position.0;
        let has_active_cast = existing_cast.is_some();

        // Face the cast target so the caster turns toward the cursor the
        // instant a cast begins, and stays pointed there while movement is
        // frozen. Self-cast spells (no target_position) keep their facing.
        //
        // This is applied AFTER validation passes so a rejected cast (on
        // cooldown, CC'd, unknown spell) does not silently rotate the player.
        let mut look_direction_value = look_direction.0;
        if let Some(target) = request.target_position {
            let offset = target - caster_position;
            let length = offset.length();
            if length > 0.001 {
                let normalized = offset / length;
                look_direction.0 = normalized;
                look_direction_value = normalized;
            }
        }

        let _ = hotbar;
        let _ = cooldowns;
        let _ = look_direction;
        let _ = existing_cast;
        let _ = cc_state;

        // If the caster already has a CastProgress, cancel it (server-side swap).
        if has_active_cast {
            commands.entity(request.caster).remove::<CastProgress>();
            emit_cast_ended(
                &mut visual_sender,
                &mut local_cast_ended,
                server,
                &caster_network_ids,
                request.caster,
                &request.spell_id,
                false,
            );
        }

        let cast_kind = spell.cast_kind();

        match cast_kind {
            CastKind::Instant => {
                let Ok(combat) = caster_stats.get(request.caster) else {
                    bevy::log::warn!("Caster {} missing CombatStats", request.caster);
                    continue;
                };

                let potential_targets: Vec<(Entity, Vec3)> = targets_query
                    .iter()
                    .filter(|(_, _, vital)| !vital.is_dead())
                    .map(|(entity, pos, _)| (entity, pos.0))
                    .collect();

                let mut ctx = SpellCastContext::new(
                    request.caster,
                    caster_position,
                    combat,
                    look_direction_value,
                    request.target_position,
                    request.target_entity,
                    &potential_targets,
                );

                spell.cast(&mut ctx);

                apply_spell_effects(
                    &mut commands,
                    &mut ctx,
                    request.caster,
                    caster_position,
                    &mut damage_events,
                    &mut heal_events,
                    &mut stat_modifier_events,
                    &mut visual_sender,
                    &mut local_visuals,
                    server,
                );

                // Cooldown starts immediately for instant spells
                if let Ok((_, mut cooldowns, _, _, _, _)) = casters.get_mut(request.caster) {
                    cooldowns
                        .start_cooldown(request.spell_id.clone(), spell_config.cooldown_seconds);
                }
            }
            CastKind::CastTime | CastKind::Channeling => {
                // Channeling starts armed so the first server tick applies the
                // effect immediately. Without this, short presses feel like F
                // does nothing and prediction lags behind the authoritative buff.
                let tick_interval_seconds = spell.channel_tick_interval_seconds();
                let channel_tick_accumulator_seconds = match cast_kind {
                    CastKind::Channeling => tick_interval_seconds,
                    _ => 0.0,
                };

                let required_seconds = match cast_kind {
                    CastKind::Channeling => spell_config.channel_duration_seconds.unwrap_or(0.0),
                    _ => spell_config.cast_time_seconds,
                };

                let movement_input_at_start = caster_inputs
                    .get(request.caster)
                    .ok()
                    .and_then(move_target_from_input);

                if matches!(cast_kind, CastKind::Channeling) {
                    if let Ok((_, mut cooldowns, _, _, _, _)) = casters.get_mut(request.caster) {
                        cooldowns.start_cooldown(
                            request.spell_id.clone(),
                            spell_config.cooldown_seconds,
                        );
                    }
                }

                commands.entity(request.caster).insert(CastProgress {
                    spell_id: request.spell_id.clone(),
                    kind: cast_kind,
                    elapsed_seconds: 0.0,
                    required_seconds,
                    channel_movement: spell_config.channel_movement,
                    last_position: caster_position,
                    target_position: request.target_position,
                    target_entity: request.target_entity,
                    movement_input_at_start,
                    channel_tick_accumulator_seconds,
                    tick_interval_seconds,
                });
            }
        }
    }
}

/// Extracts the current point-and-click movement target from replicated input.
///
/// Casts snapshot this value at start so old movement commands do not cancel a
/// newly-started CastTime; only a different target clicked during the cast does.
///
/// # Example
/// ```rust,ignore
/// let target = move_target_from_input(action_state);
/// ```
fn move_target_from_input(input: &ActionState<Inputs>) -> Option<Vec3> {
    match &input.0 {
        Inputs::MoveTo(target) => Some(*target),
        Inputs::Stop => None,
    }
}

/// Checks whether a movement command represents a new click during casting.
///
/// `None -> Some(_)` also counts as new movement: the caster was stationary
/// when the spell started and clicked afterward.
///
/// # Example
/// ```rust,ignore
/// assert!(movement_target_changed(None, Vec3::X));
/// ```
fn movement_target_changed(start_target: Option<Vec3>, current_target: Vec3) -> bool {
    let Some(start_target) = start_target else {
        return true;
    };
    start_target.distance(current_target) > MOVEMENT_INTERRUPT_EPSILON
}

/// Applies all pending effects from a [`SpellCastContext`] to the world.
/// Shared between Instant path and CastTime/Channeling completion path.
#[allow(clippy::too_many_arguments)]
fn apply_spell_effects(
    commands: &mut Commands,
    ctx: &mut SpellCastContext,
    caster: Entity,
    caster_position: Vec3,
    damage_events: &mut MessageWriter<DamageEvent>,
    heal_events: &mut MessageWriter<HealEvent>,
    stat_modifier_events: &mut MessageWriter<ApplyStatModifierEvent>,
    visual_sender: &mut ServerMultiMessageSender,
    local_visuals: &mut MessageWriter<SpellVisualEffect>,
    server: &lightyear::prelude::server::Server,
) {
    for damage_event in ctx.pending_damage.drain(..) {
        damage_events.write(damage_event);
    }
    for heal_event in ctx.pending_healing.drain(..) {
        heal_events.write(heal_event);
    }
    for proj in ctx.pending_projectiles.drain(..) {
        spawn_fireball_projectile(commands, caster_position, proj);
    }
    for aoe in ctx.pending_aoes.drain(..) {
        spawn_aoe_region(commands, caster, aoe);
    }
    for modifier in ctx.pending_modifiers.drain(..) {
        stat_modifier_events.write(modifier);
    }
    for visual in ctx.pending_visuals.drain(..) {
        send_spell_visual(visual_sender, local_visuals, server, visual);
    }
}

/// Ticks all [`CastProgress`]: advances timers, fires CastTime spells
/// upon completion, executes Channeling spell ticks, detects
/// interrupts from movement/death.
pub fn advance_cast_progress(
    time: Res<Time>,
    mut commands: Commands,
    registry: Res<SpellRegistry>,
    mut casters: Query<(
        Entity,
        &mut CastProgress,
        &Position,
        &VitalStats,
        Option<&ActionState<Inputs>>,
        Option<&mut SpellCooldowns>,
    )>,
    caster_stats: Query<(&LookDirection, &CombatStats)>,
    targets_query: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    caster_network_ids: Query<&NetworkEntityId>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut heal_events: MessageWriter<HealEvent>,
    mut stat_modifier_events: MessageWriter<ApplyStatModifierEvent>,
    mut visual_sender: ServerMultiMessageSender,
    mut local_visuals: MessageWriter<SpellVisualEffect>,
    mut local_cast_ended: MessageWriter<SpellCastEnded>,
    server: Single<&lightyear::prelude::server::Server>,
) {
    let server = server.into_inner();
    let delta = time.delta_secs();
    let mut ended: Vec<(Entity, SpellId, bool)> = Vec::new();

    for (caster_entity, mut cast, position, vital, movement_input, cooldowns) in casters.iter_mut()
    {
        // Death: always cancels.
        if vital.is_dead() {
            ended.push((caster_entity, cast.spell_id.clone(), false));
            continue;
        }

        // Movement detection (affects both CastTime and Channeling InterruptOnMove).
        let current_position = position.0;
        let moved = current_position.distance(cast.last_position) > MOVEMENT_INTERRUPT_EPSILON;

        let movement_cancels = match cast.kind {
            CastKind::CastTime => true,
            CastKind::Channeling => cast.channel_movement == ChannelMovementPolicy::InterruptOnMove,
            CastKind::Instant => false,
        };

        let movement_input_changed = movement_input
            .and_then(move_target_from_input)
            .is_some_and(|target| movement_target_changed(cast.movement_input_at_start, target));

        if movement_cancels && (moved || movement_input_changed) {
            ended.push((caster_entity, cast.spell_id.clone(), false));
            continue;
        }
        cast.last_position = current_position;

        cast.elapsed_seconds += delta;

        match cast.kind {
            CastKind::CastTime => {
                if cast.elapsed_seconds >= cast.required_seconds {
                    // Fire!
                    let end_data = fire_spell(
                        &registry,
                        &caster_stats,
                        &targets_query,
                        caster_entity,
                        current_position,
                        cast.spell_id.clone(),
                        cast.target_position,
                        cast.target_entity,
                        &mut commands,
                        &mut damage_events,
                        &mut heal_events,
                        &mut stat_modifier_events,
                        &mut visual_sender,
                        &mut local_visuals,
                        server,
                    );
                    if let Some(end_data) = end_data {
                        // Start cooldown.
                        if let Some(mut cd) = cooldowns {
                            cd.start_cooldown(cast.spell_id.clone(), end_data.cooldown_seconds);
                        }
                    }
                    ended.push((caster_entity, cast.spell_id.clone(), true));
                }
            }
            CastKind::Channeling => {
                cast.channel_tick_accumulator_seconds += delta;
                while cast.channel_tick_accumulator_seconds >= cast.tick_interval_seconds {
                    cast.channel_tick_accumulator_seconds -= cast.tick_interval_seconds;
                    let _ = fire_spell(
                        &registry,
                        &caster_stats,
                        &targets_query,
                        caster_entity,
                        current_position,
                        cast.spell_id.clone(),
                        cast.target_position,
                        cast.target_entity,
                        &mut commands,
                        &mut damage_events,
                        &mut heal_events,
                        &mut stat_modifier_events,
                        &mut visual_sender,
                        &mut local_visuals,
                        server,
                    );
                }

                if cast.required_seconds > 0.0 && cast.elapsed_seconds >= cast.required_seconds {
                    ended.push((caster_entity, cast.spell_id.clone(), true));
                }
            }
            CastKind::Instant => {
                // Defensive: should never happen, Instant would never spawn CastProgress.
                // Remove silently.
                ended.push((caster_entity, cast.spell_id.clone(), false));
            }
        }
    }

    for (caster_entity, spell_id, completed) in ended {
        commands.entity(caster_entity).remove::<CastProgress>();
        emit_cast_ended(
            &mut visual_sender,
            &mut local_cast_ended,
            server,
            &caster_network_ids,
            caster_entity,
            &spell_id,
            completed,
        );
    }
}

/// Result of a fire, used to communicate the cooldown to activate.
struct FireResult {
    cooldown_seconds: f32,
}

/// Constructs [`SpellCastContext`] for caster, executes `spell.cast(ctx)`
/// and drains effects. Returns [`FireResult`] if spell was executed
/// (i.e. was in registry and caster had CombatStats).
#[allow(clippy::too_many_arguments)]
fn fire_spell(
    registry: &SpellRegistry,
    caster_stats: &Query<(&LookDirection, &CombatStats)>,
    targets_query: &Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    caster: Entity,
    caster_position: Vec3,
    spell_id: SpellId,
    target_position: Option<Vec3>,
    target_entity: Option<Entity>,
    commands: &mut Commands,
    damage_events: &mut MessageWriter<DamageEvent>,
    heal_events: &mut MessageWriter<HealEvent>,
    stat_modifier_events: &mut MessageWriter<ApplyStatModifierEvent>,
    visual_sender: &mut ServerMultiMessageSender,
    local_visuals: &mut MessageWriter<SpellVisualEffect>,
    server: &lightyear::prelude::server::Server,
) -> Option<FireResult> {
    let spell = registry.get(&spell_id)?;
    let spell_config = spell.config();

    let Ok((look_direction, combat)) = caster_stats.get(caster) else {
        bevy::log::warn!("Caster {caster} missing LookDirection/CombatStats at fire time");
        return None;
    };

    let potential_targets: Vec<(Entity, Vec3)> = targets_query
        .iter()
        .filter(|(_, _, vital)| !vital.is_dead())
        .map(|(entity, pos, _)| (entity, pos.0))
        .collect();

    let mut ctx = SpellCastContext::new(
        caster,
        caster_position,
        combat,
        look_direction.0,
        target_position,
        target_entity,
        &potential_targets,
    );

    spell.cast(&mut ctx);

    apply_spell_effects(
        commands,
        &mut ctx,
        caster,
        caster_position,
        damage_events,
        heal_events,
        stat_modifier_events,
        visual_sender,
        local_visuals,
        server,
    );

    Some(FireResult {
        cooldown_seconds: spell_config.cooldown_seconds,
    })
}

/// Processes release requests (channeling or CastTime cancellation).
///
/// For Channeling: marks cast as completed with `completed=true`.
/// Cooldown already starts at `just_pressed`, so release does not need
/// to restart or extend it artificially. For CastTime: marks as
/// `completed=false` (cancelled).
pub fn handle_cast_release(
    mut commands: Commands,
    mut requests: MessageReader<SpellReleaseRequest>,
    casters: Query<(Entity, &CastProgress)>,
    caster_network_ids: Query<&NetworkEntityId>,
    mut visual_sender: ServerMultiMessageSender,
    mut local_cast_ended: MessageWriter<SpellCastEnded>,
    server: Single<&lightyear::prelude::server::Server>,
) {
    let server = server.into_inner();
    let mut to_end: Vec<(Entity, SpellId, bool)> = Vec::new();

    for request in requests.read() {
        let Ok((caster_entity, cast)) = casters.get(request.caster) else {
            continue;
        };
        if cast.spell_id != request.spell_id {
            continue;
        }

        let completed = matches!(cast.kind, CastKind::Channeling);
        to_end.push((caster_entity, cast.spell_id.clone(), completed));

        if completed {
            bevy::log::debug!(
                "Channeling spell {:?} released by caster {}",
                request.spell_id,
                request.caster
            );
        }
    }

    for (caster_entity, spell_id, completed) in to_end {
        commands.entity(caster_entity).remove::<CastProgress>();
        emit_cast_ended(
            &mut visual_sender,
            &mut local_cast_ended,
            server,
            &caster_network_ids,
            caster_entity,
            &spell_id,
            completed,
        );
    }
}

/// Periodically replicates progress of active casts to all clients.
/// Clients use these snapshots to update the world-space cast bar.
pub fn replicate_cast_progress(
    time: Res<Time>,
    mut elapsed_since_last_send: Local<f32>,
    casters: Query<(&CastProgress, &NetworkEntityId), With<Player>>,
    mut sender: ServerMultiMessageSender,
    mut local_progress: MessageWriter<SpellCastProgress>,
    server: Single<&lightyear::prelude::server::Server>,
) {
    *elapsed_since_last_send += time.delta_secs();
    if *elapsed_since_last_send < CAST_PROGRESS_REPLICATION_INTERVAL_SECONDS {
        return;
    }
    *elapsed_since_last_send = 0.0;

    let server = server.into_inner();
    for (cast, network_id) in casters.iter() {
        let kind_byte = match cast.kind {
            CastKind::Instant => 0,
            CastKind::CastTime => 0,
            CastKind::Channeling => 1,
        };
        let required = cast.required_seconds;
        let progress = SpellCastProgress {
            caster_network_id: network_id.0,
            spell_id: cast.spell_id.as_str().to_string(),
            kind: kind_byte,
            elapsed_seconds: cast.elapsed_seconds,
            required_seconds: required,
        };
        local_progress.write(progress.clone());
        if let Err(error) =
            sender.send::<SpellCastProgress, Channel1>(&progress, server, &NetworkTarget::All)
        {
            bevy::log::warn!("Failed to send spell cast progress: {error:?}");
        }
    }
}

fn send_spell_visual(
    sender: &mut ServerMultiMessageSender,
    local_visuals: &mut MessageWriter<SpellVisualEffect>,
    server: &lightyear::prelude::server::Server,
    visual: SpellVisualEffect,
) {
    local_visuals.write(visual.clone());
    if let Err(error) =
        sender.send::<SpellVisualEffect, Channel1>(&visual, server, &NetworkTarget::All)
    {
        bevy::log::warn!("Failed to send spell visual effect: {error:?}");
    }
}

fn emit_cast_ended(
    sender: &mut ServerMultiMessageSender,
    local_cast_ended: &mut MessageWriter<SpellCastEnded>,
    server: &lightyear::prelude::server::Server,
    caster_network_ids: &Query<&NetworkEntityId>,
    caster: Entity,
    spell_id: &SpellId,
    completed: bool,
) {
    let Some(network_id) = caster_network_ids.get(caster).ok() else {
        return;
    };
    let message = SpellCastEnded {
        caster_network_id: network_id.0,
        spell_id: spell_id.as_str().to_string(),
        completed,
    };
    local_cast_ended.write(message.clone());
    if let Err(error) =
        sender.send::<SpellCastEnded, Channel1>(&message, server, &NetworkTarget::All)
    {
        bevy::log::warn!("Failed to send spell cast ended: {error:?}");
    }
}

/// Ticks all spell cooldown timers every fixed tick.
pub fn tick_spell_cooldowns(time: Res<Time>, mut cooldowns: Query<&mut SpellCooldowns>) {
    let delta = time.delta();
    for mut cooldowns in cooldowns.iter_mut() {
        cooldowns.tick(delta);
        cooldowns.cleanup_finished();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_spell_cooldown_flow() {}
}
