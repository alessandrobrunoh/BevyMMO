//! Server-authoritative systems per il pipeline di cast delle spell.
//!
//! Tre modelli temporali sono supportati:
//! - [`CastKind::Instant`]: effetto immediato alla pressione del tasto.
//! - [`CastKind::CastTime`]: wind-up bloccante, movimento cancella sempre.
//! - [`CastKind::Channeling`]: effetto tick-ripetuto finché rilasciato,
//!   movement policy configurabile per-spell.

use bevy::prelude::*;
use std::sync::Arc;

use crate::network::protocol::{
    Channel1, LookDirection, NetworkEntityId, Position, SpellCastEnded, SpellCastProgress,
    SpellVisualEffect,
};
use crate::plugins::entity::components::GameEntity;
use crate::plugins::entity::player::Player;
use crate::stats::components::{CombatStats, VitalStats};
use crate::stats::events::{ApplyStatModifierEvent, DamageEvent, HealEvent};
use lightyear::prelude::{NetworkTarget, ServerMultiMessageSender};

use super::{
    aoe::spawn_aoe_region,
    components::{CastProgress, SpellCooldowns, Spellbook, MOVEMENT_INTERRUPT_EPSILON},
    context::{CastKind, Spell, SpellCastContext},
    events::{SpellCastRequest, SpellReleaseRequest},
    registry::SpellRegistry,
};
use crate::plugins::spells::SpellId;

/// Processa le richieste di cast dai client.
///
/// Le spell Instant vengono eseguite immediatamente (comportamento storico).
/// Le spell CastTime/Channeling non eseguono `cast` qui: spawnano un componente
/// [`CastProgress`] sul caster, poi il sistema [`advance_cast_progress`] lo
/// ticka fino al completamento. Prima di spawnare un nuovo `CastProgress`,
/// qualsiasi cast già attivo sul caster viene cancellato con emissione di
/// [`SpellCastEnded`].
pub fn process_cast_requests(
    mut commands: Commands,
    mut requests: MessageReader<SpellCastRequest>,
    registry: Res<SpellRegistry>,
    mut casters: Query<(
        &Spellbook,
        &mut SpellCooldowns,
        &Position,
        Option<&CastProgress>,
    )>,
    caster_stats: Query<(&LookDirection, &CombatStats)>,
    caster_network_ids: Query<&NetworkEntityId>,
    targets_query: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut heal_events: MessageWriter<HealEvent>,
    mut stat_modifier_events: MessageWriter<ApplyStatModifierEvent>,
    mut visual_sender: ServerMultiMessageSender,
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

        // Step 2: validate spellbook + cooldowns
        let Ok((spellbook, mut cooldowns, caster_position, existing_cast)) =
            casters.get_mut(request.caster)
        else {
            bevy::log::warn!("Caster {} missing spell state", request.caster);
            continue;
        };

        if !spellbook.contains(&request.spell_id) {
            bevy::log::warn!(
                "Caster {} lacks spell {:?} in spellbook",
                request.caster,
                request.spell_id
            );
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

        // Drop the borrow before potentially spawning CastProgress / firing.
        let caster_position = caster_position.0;
        let has_active_cast = existing_cast.is_some();
        drop(spellbook);
        drop(cooldowns);
        drop(existing_cast);

        // If the caster already has a CastProgress, cancel it (server-side swap).
        if has_active_cast {
            commands.entity(request.caster).remove::<CastProgress>();
            emit_cast_ended(
                &mut visual_sender,
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
                let Ok((look_direction, combat)) = caster_stats.get(request.caster) else {
                    bevy::log::warn!(
                        "Caster {} missing LookDirection/CombatStats",
                        request.caster
                    );
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
                    look_direction.0,
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
                    server,
                );

                // Cooldown parte subito per le spell instant
                if let Ok((_, mut cooldowns, _, _)) = casters.get_mut(request.caster) {
                    cooldowns
                        .start_cooldown(request.spell_id.clone(), spell_config.cooldown_seconds);
                }
            }
            CastKind::CastTime | CastKind::Channeling => {
                // Spawn del CastProgress. Nessun effetto qui; advance_cast_progress
                // decide quando lanciare la spell in base al kind.
                let tick_interval = spell.channel_tick_interval_seconds();
                commands.entity(request.caster).insert(CastProgress {
                    spell_id: request.spell_id.clone(),
                    kind: cast_kind,
                    elapsed_seconds: 0.0,
                    required_seconds: spell_config.cast_time_seconds,
                    channel_movement: spell_config.channel_movement,
                    last_position: caster_position,
                    target_position: request.target_position,
                    target_entity: request.target_entity,
                    channel_tick_accumulator_seconds: 0.0,
                    tick_interval_seconds: tick_interval,
                });
            }
        }
    }
}

/// Applica tutti gli effetti pendenti di uno [`SpellCastContext`] al mondo.
/// Condiviso tra il path Instant e quello CastTime/Channeling completion.
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
    server: &lightyear::prelude::server::Server,
) {
    for damage_event in ctx.pending_damage.drain(..) {
        damage_events.write(damage_event);
    }
    for heal_event in ctx.pending_healing.drain(..) {
        heal_events.write(heal_event);
    }
    for proj in ctx.pending_projectiles.drain(..) {
        crate::spells::followball::projectile::spawn(commands, caster_position, proj);
    }
    for aoe in ctx.pending_aoes.drain(..) {
        spawn_aoe_region(commands, caster, aoe);
    }
    for modifier in ctx.pending_modifiers.drain(..) {
        stat_modifier_events.write(modifier);
    }
    for visual in ctx.pending_visuals.drain(..) {
        send_spell_visual(visual_sender, server, visual);
    }
}

/// Ticka tutti i [`CastProgress`]: fa avanzare i timer, lancia le spell
/// CastTime al completamento, esegue i tick delle spell Channeling, rileva
/// interruzioni da movimento/morte.
pub fn advance_cast_progress(
    time: Res<Time>,
    mut commands: Commands,
    registry: Res<SpellRegistry>,
    mut casters: Query<(
        Entity,
        &mut CastProgress,
        &Position,
        &VitalStats,
        Option<&mut SpellCooldowns>,
    )>,
    caster_stats: Query<(&LookDirection, &CombatStats)>,
    targets_query: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    caster_network_ids: Query<&NetworkEntityId>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut heal_events: MessageWriter<HealEvent>,
    mut stat_modifier_events: MessageWriter<ApplyStatModifierEvent>,
    mut visual_sender: ServerMultiMessageSender,
    server: Single<&lightyear::prelude::server::Server>,
) {
    let server = server.into_inner();
    let delta = time.delta_secs();
    let mut ended: Vec<(Entity, SpellId, bool)> = Vec::new();

    for (caster_entity, mut cast, position, vital, cooldowns) in casters.iter_mut() {
        // Morte: cancella sempre.
        if vital.is_dead() {
            ended.push((caster_entity, cast.spell_id.clone(), false));
            continue;
        }

        // Movement detection (raggiunge sia CastTime che Channeling InterruptOnMove).
        let current_position = position.0;
        let moved = current_position.distance(cast.last_position) > MOVEMENT_INTERRUPT_EPSILON;

        let movement_cancels = match cast.kind {
            CastKind::CastTime => true,
            CastKind::Channeling => {
                cast.channel_movement
                    == crate::plugins::spells::ChannelMovementPolicy::InterruptOnMove
            }
            CastKind::Instant => false,
        };

        if moved && movement_cancels {
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
                        server,
                    );
                }
            }
            CastKind::Instant => {
                // Defensive: non dovrebbe mai capitare, un Instant non spawnerebbe
                // mai un CastProgress. Rimuovilo silenziosamente.
                ended.push((caster_entity, cast.spell_id.clone(), false));
            }
        }
    }

    for (caster_entity, spell_id, completed) in ended {
        commands.entity(caster_entity).remove::<CastProgress>();
        emit_cast_ended(
            &mut visual_sender,
            server,
            &caster_network_ids,
            caster_entity,
            &spell_id,
            completed,
        );
    }
}

/// Risultato di un fire, usato per comunicare il cooldown da attivare.
struct FireResult {
    cooldown_seconds: f32,
}

/// Costruisce lo [`SpellCastContext`] per il caster, esegue `spell.cast(ctx)`
/// e drena gli effetti. Ritorna [`FireResult`] se la spell è stata eseguita
/// (cioè era nel registry e il caster aveva CombatStats).
#[allow(clippy::too_many_arguments)]
fn fire_spell(
    registry: &SpellRegistry,
    caster_stats: &Query<(&LookDirection, &CombatStats)>,
    targets_query: &Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    caster: Entity,
    caster_position: Vec3,
    spell_id: crate::plugins::spells::SpellId,
    target_position: Option<Vec3>,
    target_entity: Option<Entity>,
    commands: &mut Commands,
    damage_events: &mut MessageWriter<DamageEvent>,
    heal_events: &mut MessageWriter<HealEvent>,
    stat_modifier_events: &mut MessageWriter<ApplyStatModifierEvent>,
    visual_sender: &mut ServerMultiMessageSender,
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
        server,
    );

    Some(FireResult {
        cooldown_seconds: spell_config.cooldown_seconds,
    })
}

/// Processa le richieste di rilascio (channeling o cancellazione CastTime).
///
/// Per Channeling: marca il cast come terminato con `completed=true` e attiva
/// il cooldown. Per CastTime: marca come `completed=false` (cancellato).
pub fn handle_cast_release(
    mut commands: Commands,
    mut requests: MessageReader<SpellReleaseRequest>,
    casters: Query<(Entity, &CastProgress)>,
    caster_network_ids: Query<&NetworkEntityId>,
    registry: Res<SpellRegistry>,
    mut cooldowns_query: Query<&mut SpellCooldowns>,
    mut visual_sender: ServerMultiMessageSender,
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

        // Per Channeling avviamo il cooldown al rilascio.
        if completed {
            if let Some(spell) = registry.get(&request.spell_id) {
                if let Ok(mut cd) = cooldowns_query.get_mut(request.caster) {
                    cd.start_cooldown(request.spell_id.clone(), spell.config().cooldown_seconds);
                }
            }
        }
    }

    for (caster_entity, spell_id, completed) in to_end {
        commands.entity(caster_entity).remove::<CastProgress>();
        emit_cast_ended(
            &mut visual_sender,
            server,
            &caster_network_ids,
            caster_entity,
            &spell_id,
            completed,
        );
    }
}

/// Replica lo stato dei cast in corso a tutti i client ogni tick.
/// I client usano questi snapshot per aggiornare la barra world-space.
pub fn replicate_cast_progress(
    casters: Query<(&CastProgress, &NetworkEntityId), With<Player>>,
    mut sender: ServerMultiMessageSender,
    server: Single<&lightyear::prelude::server::Server>,
) {
    let server = server.into_inner();
    for (cast, network_id) in casters.iter() {
        let kind_byte = match cast.kind {
            CastKind::Instant => 0,
            CastKind::CastTime => 0,
            CastKind::Channeling => 1,
        };
        let required = match cast.kind {
            CastKind::Channeling => 0.0,
            _ => cast.required_seconds,
        };
        let progress = SpellCastProgress {
            caster_network_id: network_id.0,
            spell_id: cast.spell_id.as_str().to_string(),
            kind: kind_byte,
            elapsed_seconds: cast.elapsed_seconds,
            required_seconds: required,
        };
        if let Err(error) =
            sender.send::<SpellCastProgress, Channel1>(&progress, server, &NetworkTarget::All)
        {
            bevy::log::warn!("Failed to send spell cast progress: {error:?}");
        }
    }
}

fn send_spell_visual(
    sender: &mut ServerMultiMessageSender,
    server: &lightyear::prelude::server::Server,
    visual: SpellVisualEffect,
) {
    if let Err(error) =
        sender.send::<SpellVisualEffect, Channel1>(&visual, server, &NetworkTarget::All)
    {
        bevy::log::warn!("Failed to send spell visual effect: {error:?}");
    }
}

fn emit_cast_ended(
    sender: &mut ServerMultiMessageSender,
    server: &lightyear::prelude::server::Server,
    caster_network_ids: &Query<&NetworkEntityId>,
    caster: Entity,
    spell_id: &crate::plugins::spells::SpellId,
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

/// Register all built-in spells at startup.
pub fn register_builtin_spells(mut registry: ResMut<SpellRegistry>) {
    bevy::log::info!("Registering built-in spells...");

    let attack_spell: Arc<dyn Spell> = Arc::new(crate::spells::attack::AttackSpell);
    registry.register(attack_spell);

    let fireball_spell: Arc<dyn Spell> = Arc::new(crate::spells::fireball::FireballSpell);
    registry.register(fireball_spell);

    let followball_spell: Arc<dyn Spell> = Arc::new(crate::spells::followball::FollowballSpell);
    registry.register(followball_spell);

    let healing_circle_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::healing_circle::definition::HealingCircleSpell);
    registry.register(healing_circle_spell);

    let meteorite_spell: Arc<dyn Spell> = Arc::new(crate::spells::meteorite::MeteoriteSpell);
    registry.register(meteorite_spell);

    let swift_spell: Arc<dyn Spell> = Arc::new(crate::spells::swift::SwiftSpell);
    registry.register(swift_spell);

    bevy::log::info!("Registered {} built-in spells", registry.len());
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_spell_cooldown_flow() {}
}
