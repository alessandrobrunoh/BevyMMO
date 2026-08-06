//! Regioni AoE persistenti e loro ciclo di vita server-authoritative.
//!
//! Modulo parallelo a [`crate::plugins::spells::projectile`]: gestisce il
//! ciclo di vita di qualsiasi entità dotata del componente [`AoeRegion`]. Il
//! payload dell'effetto (modifier, danno, cura, targeting) è portato dal
//! componente stesso, quindi il sistema è del tutto agnostico rispetto alla
//! spell che ha spawnato la regione.

use bevy::prelude::*;
use std::collections::HashSet;

use crate::network::protocol::Position;
use crate::plugins::crowd_control::ApplyCrowdControlEvent;
use crate::plugins::entity::components::GameEntity;
use crate::plugins::spells::context::{AoeEffect, AoeSpawnRequest};
use crate::stats::components::VitalStats;
use crate::stats::events::{ApplyStatModifierEvent, DamageEvent, HealEvent};

/// Componente per una regione ad area (AoE) persistente.
#[derive(Component)]
pub struct AoeRegion {
    pub caster: Entity,
    pub center: Vec3,
    pub radius: f32,
    /// Tempo di vita residuo della regione. Quando arriva a 0 la regione
    /// despawna (dopo aver eventualmente applicato un effetto finale).
    pub remaining_seconds: f32,
    pub spell_id: String,
    /// Tempo di delay prima che l'effetto venga applicato la prima volta.
    /// During this window the region exists (es. cerchio rosso del Meteorite)
    /// ma non applica damage/heal/modifier.
    pub pending_delay_seconds: f32,
    /// Quali entità hanno già ricevuto l'effetto (usato quando
    /// `AoeEffect::ApplyModifier.once_per_entity` è `true`).
    pub affected_entities: HashSet<Entity>,
    /// Payload dell'effetto da applicare alle entità che entrano nell'area.
    /// Il sistema centrale legge questo campo invece di fare dispatch su
    /// `spell_id`.
    pub effect: AoeEffect,
}

pub fn spawn_aoe_region(commands: &mut Commands, caster: Entity, req: AoeSpawnRequest) {
    commands.spawn(AoeRegion {
        caster,
        center: req.center,
        radius: req.radius,
        remaining_seconds: req.duration_seconds,
        pending_delay_seconds: req.initial_delay_seconds.max(0.0),
        spell_id: req.spell_id,
        affected_entities: HashSet::new(),
        effect: req.effect,
    });
}

/// Sistema server-authoritative che gestisce il ciclo di vita delle regioni AoE
/// e applica gli effetti alle entità all'interno del raggio.
///
/// È completamente generico rispetto alla spell: legge `region.effect` (un
/// [`AoeEffect`] passato dalla spell al momento del cast) e non fa dispatch su
/// `spell_id`. Aggiungere una nuova spell ad area (es. poison cloud) richiede
/// solo un nuovo file in `src/spells/` che emetta un `AoeEffect` opportuno.
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
        // visible (useful per il cerchio di warning del Meteorite) ma non
        // applica alcun effetto.
        if region.pending_delay_seconds > 0.0 {
            region.pending_delay_seconds = (region.pending_delay_seconds - delta).max(0.0);
        }
        region.remaining_seconds -= delta;

        // Applica l'effetto solo se il delay iniziale è trascorso.
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

        // Despawn alla scadenza. Per gli effetti Damage/Heal con delay
        // (Meteorite) questo avviene subito dopo il singolo tick di impatto.
        if region.remaining_seconds <= 0.0 {
            regions_to_despawn.push(region_entity);
        }
    }

    for entity in regions_to_despawn {
        commands.entity(entity).despawn();
    }
}

/// Applica l'effetto della regione a tutte le entità idonee attualmente
/// all'interno del raggio. Estratto per leggibilità e per poterlo saltare
/// durante la fase di delay.
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

        let distance = target_pos.0.distance(region.center);
        if distance > region.radius {
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
                // Per i burst Damage assumiamo semantica "once_per_entity":
                // tracciamo per non riapplicare finché la regione vive (di
                // solito despawna subito dopo, ma in caso di durata > 0 serve).
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
