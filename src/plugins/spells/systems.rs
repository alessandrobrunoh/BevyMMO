//! Systems for processing spell cast requests and registering built-in spells.

use bevy::prelude::*;
use std::sync::Arc;

use crate::network::protocol::{Channel1, LookDirection, Position, SpellVisualEffect};
use crate::plugins::entity::components::GameEntity;
use crate::stats::components::{CombatStats, VitalStats};
use crate::stats::events::{DamageEvent, HealEvent};
use lightyear::prelude::{NetworkTarget, ServerMultiMessageSender};

use super::{
    aoe::spawn_aoe_region,
    components::{SpellCooldowns, Spellbook},
    context::{Spell, SpellCastContext},
    events::SpellCastRequest,
    registry::SpellRegistry,
};

/// Process spell cast requests from clients.
///
/// This server-only system:
/// 1. Reads all pending spell cast requests
/// 2. Validates the request (spell exists, caster has spell, cooldown ready)
/// 3. Executes the spell logic
/// 4. Emits damage/healing events
/// 5. Resets cooldowns
///
/// Runs in FixedUpdate to match the game's fixed timestep.
pub fn process_cast_requests(
    mut commands: Commands,
    mut requests: MessageReader<SpellCastRequest>,
    registry: Res<SpellRegistry>,
    mut spell_state_query: Query<(&Spellbook, &mut SpellCooldowns)>,
    caster_query: Query<(&Position, &LookDirection, &CombatStats)>,
    targets_query: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut heal_events: MessageWriter<HealEvent>,
    mut visual_sender: ServerMultiMessageSender,
    server: Single<&lightyear::prelude::server::Server>,
) {
    let server = server.into_inner();
    for request in requests.read() {
        // Step 1: Look up the spell in the registry
        let spell = match registry.get(&request.spell_id) {
            Some(spell) => spell,
            None => {
                bevy::log::warn!(
                    "Spell cast request for unknown spell: {:?}",
                    request.spell_id
                );
                continue;
            }
        };

        let spell_config = spell.config();

        // Step 2: validate spellbook and cooldown state.
        let (spellbook, mut cooldowns) = match spell_state_query.get_mut(request.caster) {
            Ok(state) => state,
            Err(_) => {
                bevy::log::warn!(
                    "Caster {} has no Spellbook/SpellCooldowns component",
                    request.caster
                );
                continue;
            }
        };

        if !spellbook.contains(&request.spell_id) {
            bevy::log::warn!(
                "Caster {} tried to cast unknown spell from spellbook: {:?}",
                request.caster,
                request.spell_id
            );
            continue;
        }

        // Step 3: Check if spell is ready.
        if cooldowns.is_on_cooldown(&request.spell_id) {
            bevy::log::debug!(
                "Spell {:?} is on cooldown for caster {}",
                request.spell_id,
                request.caster
            );
            continue;
        }

        // Step 4: Get caster data (position and combat stats)
        let (caster_position, caster_look_direction, caster_combat) =
            match caster_query.get(request.caster) {
                Ok((pos, look_direction, combat)) => (pos.0, look_direction.0, combat),
                Err(_) => {
                    bevy::log::warn!(
                        "Caster {} has no Position, LookDirection or CombatStats",
                        request.caster
                    );
                    continue;
                }
            };

        // Step 5: Collect potential targets (all living GameEntity)
        let potential_targets: Vec<(Entity, Vec3)> = targets_query
            .iter()
            .filter(|(_, _, vital)| !vital.is_dead())
            .map(|(entity, pos, _)| (entity, pos.0))
            .collect();

        // Step 6: Build the spell cast context
        let mut ctx = SpellCastContext::new(
            request.caster,
            caster_position,
            caster_combat,
            caster_look_direction,
            request.target_position,
            request.target_entity,
            &potential_targets,
        );

        // Step 7: Execute the spell
        spell.cast(&mut ctx);

        // Step 8: Drain pending events into the actual event writers
        for damage_event in ctx.pending_damage {
            damage_events.write(damage_event);
        }

        for heal_event in ctx.pending_healing {
            heal_events.write(heal_event);
        }

        // Step 10: Spawn homing projectiles
        for proj in ctx.pending_projectiles {
            crate::spells::followball::projectile::spawn(&mut commands, caster_position, proj);
        }

        // Step 11: Spawn AoE regions
        for aoe in ctx.pending_aoes {
            spawn_aoe_region(&mut commands, request.caster, aoe);
        }

        for visual in ctx.pending_visuals {
            send_spell_visual(&mut visual_sender, server, visual);
        }

        // Step 11: Start the cooldown
        cooldowns.start_cooldown(request.spell_id.clone(), spell_config.cooldown_seconds);
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

/// Ticks all spell cooldown timers every fixed tick.
pub fn tick_spell_cooldowns(time: Res<Time>, mut cooldowns: Query<&mut SpellCooldowns>) {
    let delta = time.delta();
    for mut cooldowns in cooldowns.iter_mut() {
        cooldowns.tick(delta);
        cooldowns.cleanup_finished();
    }
}

/// Register all built-in spells at startup.
///
/// This system runs once during the Startup schedule to populate the
/// SpellRegistry with all game-defined spells.
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

    bevy::log::info!("Registered {} built-in spells", registry.len());
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_spell_cooldown_flow() {}
}
