//! Persistent AoE regions and their server-authoritative lifecycle.
//!
//! Parallel module to [`crate::plugins::spells::projectile`]: manages the
//! lifecycle of any entity possessing an [`AoeRegion`] component. The
//! effect payload (modifier, damage, heal, targeting) is carried by the
//! component itself, so the system is entirely agnostic to the
//! spell that spawned the region.

use bevy::prelude::*;
use std::collections::HashSet;

use bevymmo_shared::entity::components::GameEntity;
use bevymmo_shared::network::protocol::Position;
use bevymmo_shared::spells::context::{AoeEffect, AoeShape, AoeSpawnRequest};
use bevymmo_shared::stats::components::VitalStats;
use bevymmo_shared::stats::events::{ApplyStatModifierEvent, DamageEvent, HealEvent};

use crate::crowd_control::ApplyCrowdControlEvent;

/// Component for a persistent Area-of-Effect (AoE) region.
#[derive(Component)]
pub struct AoeRegion {
    pub caster: Entity,
    pub center: Vec3,
    pub radius: f32,
    /// Forma coperta attorno a `center`. `Circle` per tutte le spell
    /// classiche; i gesti Eidolon a cono portano `Cone`, ed è la stessa
    /// forma che il client disegna in anteprima prima del lancio.
    pub shape: AoeShape,
    /// Remaining lifetime of the region. When it reaches 0 the region
    /// despawns (after optionally applying a final effect).
    pub remaining_seconds: f32,
    pub spell_id: String,
    /// Delay time before the effect is applied for the first time.
    /// During this window the region exists (e.g. Meteorite's red circle)
    /// but does not apply damage/heal/modifier.
    pub pending_delay_seconds: f32,
    /// Which entities have already received the effect (used when
    /// `AoeEffect::ApplyModifier.once_per_entity` is `true`).
    pub affected_entities: HashSet<Entity>,
    /// Effect payload to apply to entities entering the area.
    /// The central system reads this field instead of dispatching on
    /// `spell_id`.
    pub effect: AoeEffect,
}

pub fn spawn_aoe_region(commands: &mut Commands, caster: Entity, req: AoeSpawnRequest) {
    commands.spawn(AoeRegion {
        caster,
        center: req.center,
        radius: req.radius,
        shape: req.shape,
        remaining_seconds: req.duration_seconds,
        pending_delay_seconds: req.initial_delay_seconds.max(0.0),
        spell_id: req.spell_id,
        affected_entities: HashSet::new(),
        effect: req.effect,
    });
}

/// Server-authoritative system that manages the lifecycle of AoE regions
/// and applies effects to entities within range.
///
/// It is completely generic with respect to the spell: it reads `region.effect` (an
/// [`AoeEffect`] passed by the spell at cast time) and does not dispatch on
/// `spell_id`. Adding a new area spell (e.g. poison cloud) only requires
/// a new file in `src/spells/` that emits an appropriate `AoeEffect`.
pub fn update_aoe_regions(
    time: Res<Time>,
    mut commands: Commands,
    mut regions: Query<(Entity, &mut AoeRegion)>,
    targets: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    mut stat_modifiers: MessageWriter<ApplyStatModifierEvent>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut heal_events: MessageWriter<HealEvent>,
    mut cc_events: MessageWriter<ApplyCrowdControlEvent>,
) {
    let delta = time.delta_secs();
    let mut regions_to_despawn: Vec<Entity> = Vec::new();

    for (region_entity, mut region) in regions.iter_mut() {
        // Tick down delay first. While the delay is active the region is
        // visible (useful for Meteorite's warning circle) but
        // applies no effect.
        if region.pending_delay_seconds > 0.0 {
            region.pending_delay_seconds = (region.pending_delay_seconds - delta).max(0.0);
        }
        region.remaining_seconds -= delta;

        // Apply effect only if the initial delay has elapsed.
        let effect_armed = region.pending_delay_seconds <= 0.0;
        if effect_armed {
            apply_aoe_effect_to_targets(
                &mut region,
                &targets,
                &mut stat_modifiers,
                &mut damage_events,
                &mut heal_events,
                &mut cc_events,
            );
        }

        // Despawn on expiry. For Damage/Heal effects with delay
        // (Meteorite) this happens right after the single impact tick.
        if region.remaining_seconds <= 0.0 {
            regions_to_despawn.push(region_entity);
        }
    }

    for entity in regions_to_despawn {
        commands.entity(entity).despawn();
    }
}

/// Applies region effect to all eligible entities currently
/// inside the radius. Extracted for readability and to allow skipping it
/// during the delay phase.
#[allow(clippy::too_many_arguments)]
fn apply_aoe_effect_to_targets(
    region: &mut AoeRegion,
    targets: &Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
    stat_modifiers: &mut MessageWriter<ApplyStatModifierEvent>,
    damage_events: &mut MessageWriter<DamageEvent>,
    heal_events: &mut MessageWriter<HealEvent>,
    cc_events: &mut MessageWriter<ApplyCrowdControlEvent>,
) {
    let targeting = region.effect.targeting();

    for (target_entity, target_pos, target_vital) in targets.iter() {
        if target_vital.is_dead() {
            continue;
        }
        if !targeting.allows(region.caster, target_entity) {
            continue;
        }

        if !region.shape.contains(region.center, region.radius, target_pos.0) {
            continue;
        }

        match &region.effect {
            AoeEffect::ApplyModifier {
                effects,
                duration_seconds,
                kind,
                once_per_entity,
                targeting: _,
            } => {
                if *once_per_entity && region.affected_entities.contains(&target_entity) {
                    continue;
                }

                stat_modifiers.write(ApplyStatModifierEvent {
                    target: target_entity,
                    source: Some(region.caster),
                    effects: effects.clone(),
                    duration_seconds: *duration_seconds,
                    kind: *kind,
                });

                if *once_per_entity {
                    region.affected_entities.insert(target_entity);
                }
            }
            AoeEffect::Damage {
                amount,
                targeting: _,
            } => {
                // For burst Damage we assume "once_per_entity" semantics:
                // track to avoid re-applying while the region lives (usually
                // despawns immediately after, but needed in case duration > 0).
                if region.affected_entities.contains(&target_entity) {
                    continue;
                }
                damage_events.write(DamageEvent {
                    target: target_entity,
                    source: Some(region.caster),
                    amount: *amount,
                });
                region.affected_entities.insert(target_entity);
            }

            AoeEffect::Heal {
                amount,
                targeting: _,
            } => {
                if region.affected_entities.contains(&target_entity) {
                    continue;
                }
                heal_events.write(HealEvent {
                    target: target_entity,
                    source: Some(region.caster),
                    amount: *amount,
                });
                region.affected_entities.insert(target_entity);
            }
            AoeEffect::CrowdControl {
                kind,
                duration_seconds,
                once_per_entity,
                targeting: _,
            } => {
                if *once_per_entity && region.affected_entities.contains(&target_entity) {
                    continue;
                }
                cc_events.write(ApplyCrowdControlEvent {
                    target: target_entity,
                    source: Some(region.caster),
                    kind: *kind,
                    duration_seconds: *duration_seconds,
                });
                if *once_per_entity {
                    region.affected_entities.insert(target_entity);
                }
            }
        }
    }
}
