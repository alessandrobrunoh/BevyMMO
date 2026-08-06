//! Systems for processing spell cast requests and registering built-in spells.

use bevy::prelude::*;
use std::sync::Arc;

use crate::network::protocol::{EntityColor, Position, SpellVisualEffect};
use crate::plugins::entity::components::GameEntity;
use crate::stats::components::{CombatStats, VitalStats};
use crate::stats::events::{DamageEvent, HealEvent};
use lightyear::prelude::{NetworkTarget, Replicate};

use super::{
    components::{SpellCooldowns, Spellbook},
    context::{ProjectileSpawnRequest, Spell, SpellCastContext},
    events::SpellCastRequest,
    registry::SpellRegistry,
};

/// Component marker for a homing projectile entity.
#[derive(Component, Debug)]
pub struct HomingProjectile {
    pub target: Entity,
    pub speed: f32,
    pub damage: f32,
    pub hit_radius: f32,
}

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
    caster_query: Query<(&Position, &CombatStats)>,
    targets_query: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut heal_events: MessageWriter<HealEvent>,
    mut visual_effects: MessageWriter<SpellVisualEffect>,
) {
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
        let (caster_position, caster_combat) = match caster_query.get(request.caster) {
            Ok((pos, combat)) => (pos.0, combat),
            Err(_) => {
                bevy::log::warn!("Caster {} has no Position or CombatStats", request.caster);
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

        // Step 9: Broadcast visual effects to all clients
        for visual in ctx.pending_visuals {
            visual_effects.write(visual);
        }

        // Step 10: Spawn homing projectiles
        for proj in ctx.pending_projectiles {
            spawn_homing_projectile(&mut commands, caster_position, proj);
        }

        // Step 11: Start the cooldown
        cooldowns.start_cooldown(request.spell_id.clone(), spell_config.cooldown_seconds);
    }
}

/// Spawna una entity projectile replicata con Position + EntityColor + HomingProjectile.
fn spawn_homing_projectile(commands: &mut Commands, start: Vec3, proj: ProjectileSpawnRequest) {
    commands.spawn((
        Position(start),
        EntityColor(Color::srgb(0.2, 0.8, 1.0)),
        HomingProjectile {
            target: proj.target,
            speed: proj.speed,
            damage: proj.damage,
            hit_radius: proj.hit_radius,
        },
        Replicate::to_clients(NetworkTarget::All),
    ));
}

/// Sistema server-authoritative: muove i proiettili homing verso il target
/// e applica danno all'impatto.
///
/// Le due query sono rese disgiunte da `Without<HomingProjectile>`: i target
/// non possono essere proiettili stessi, evitando il conflitto B0001.
pub fn update_homing_projectiles(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Position, &HomingProjectile)>,
    targets: Query<&Position, Without<HomingProjectile>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (proj_entity, mut proj_pos, proj) in projectiles.iter_mut() {
        // Se il target non esiste più o non ha Position, despawn
        let Ok(target_pos) = targets.get(proj.target) else {
            commands.entity(proj_entity).despawn();
            continue;
        };

        let direction = target_pos.0 - proj_pos.0;
        let distance = direction.length();

        // Hit: abbastanza vicino da colpire
        if distance <= proj.hit_radius {
            damage_events.write(DamageEvent {
                target: proj.target,
                source: None,
                amount: proj.damage,
            });
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Muovi verso il target
        let step = proj.speed.min(distance);
        proj_pos.0 += direction / distance * step;
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

    bevy::log::info!("Registered {} built-in spells", registry.len());
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_spell_cooldown_flow() {}
}
