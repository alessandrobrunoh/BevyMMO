use bevy::prelude::*;
use std::collections::HashSet;

use crate::network::protocol::Position;
use crate::plugins::entity::components::GameEntity;
use crate::plugins::spells::context::{AoeEffect, AoeSpawnRequest};
use crate::stats::components::VitalStats;
use crate::stats::events::ApplyStatModifierEvent;

/// Componente per una regione ad area (AoE) persistente.
#[derive(Component)]
pub struct AoeRegion {
    pub caster: Entity,
    pub center: Vec3,
    pub radius: f32,
    pub remaining_seconds: f32,
    pub spell_id: String,
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
) {
    let delta = time.delta_secs();

    for (region_entity, mut region) in regions.iter_mut() {
        region.remaining_seconds -= delta;
        if region.remaining_seconds <= 0.0 {
            commands.entity(region_entity).despawn();
            continue;
        }

        for (target_entity, target_pos, target_vital) in targets.iter() {
            if target_vital.is_dead() {
                continue;
            }

            let distance = target_pos.0.distance(region.center);
            if distance > region.radius {
                continue;
            }

            // Applica l'effetto descritto dalla spell nel payload `region.effect`.
            match &region.effect {
                AoeEffect::ApplyModifier {
                    effects,
                    duration_seconds,
                    kind,
                    once_per_entity,
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
            }
        }
    }
}
