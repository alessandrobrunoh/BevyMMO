//! Derived ability blueprint shared by preview and execution.
//!
//! A blueprint is not persisted. It is rebuilt from the base ability, the item
//! and later the item's Root Words/Ancient Words. Persisted state contains only
//! stable ids and selections.

use super::base_ability::{
    AbilityCastMode, AbilityGeometry, AbilityId, AbilityParams, AbilityTag, BaseAbility,
};
use crate::effects::{ApplyStatusEffect, DamageEffect, EffectSpec, HealEffect, StatusId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintExecution {
    Base,
    Charge,
    /// The item repeats an eligible manifestation once after the original.
    Echo,
}

/// What the Root Word makes the gesture *do*. Geometry stays on the ability;
/// this is the payload written by `RootWordEffect::apply_to_blueprint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ManifestationKind {
    #[default]
    Damage,
    Heal,
}

/// Neutral effect identity carried through preview and authoritative cast.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ManifestationPayload {
    pub kind: ManifestationKind,
    pub status_ids: Vec<StatusId>,
}

impl ManifestationPayload {
    pub fn damage(status_ids: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            kind: ManifestationKind::Damage,
            status_ids: status_ids.into_iter().map(StatusId::new).collect(),
        }
    }

    pub fn heal(status_ids: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            kind: ManifestationKind::Heal,
            status_ids: status_ids.into_iter().map(StatusId::new).collect(),
        }
    }

    /// Effect specs for this payload at `potency`. Statuses are applied after
    /// the primary hit so a burn lands on a living target that just took damage.
    pub fn effect_specs(&self, potency: f32) -> Vec<EffectSpec> {
        let mut effects = vec![match self.kind {
            ManifestationKind::Damage => EffectSpec::Damage(DamageEffect { amount: potency }),
            ManifestationKind::Heal => EffectSpec::Heal(HealEffect { amount: potency }),
        }];
        for status_id in &self.status_ids {
            effects.push(EffectSpec::ApplyStatus(ApplyStatusEffect {
                status_id: status_id.clone(),
                duration_override_seconds: None,
                potency: 1.0,
            }));
        }
        effects
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityBlueprint {
    pub ability_id: AbilityId,
    pub tags: Vec<AbilityTag>,
    pub geometry: AbilityGeometry,
    pub cast_mode: AbilityCastMode,
    pub execution: BlueprintExecution,
    pub params: AbilityParams,
    pub animation: &'static str,
    pub impact_vfx: &'static str,
    pub impact_delay: f32,
    pub stun_seconds: f32,
    /// Written by the Root Word. Empty/damage is the neutral pre-root state.
    pub payload: ManifestationPayload,
}

impl AbilityBlueprint {
    pub fn from_base_ability<T: BaseAbility + ?Sized>(ability: &T) -> Self {
        Self {
            ability_id: ability.id(),
            tags: ability.tags().to_vec(),
            geometry: ability.geometry(),
            cast_mode: ability.cast_mode(),
            execution: BlueprintExecution::Base,
            params: ability.base_params(),
            animation: ability.animation(),
            impact_vfx: ability.impact_vfx(),
            impact_delay: ability.impact_delay(),
            stun_seconds: ability.stun_seconds(),
            payload: ManifestationPayload::default(),
        }
    }

    pub fn has_tag(&self, tag: AbilityTag) -> bool {
        self.tags.contains(&tag)
    }

    /// Effects the authoritative cast must emit: Root Word payload, plus the
    /// gesture's impact stun when this is still a damaging hit.
    pub fn payload_effects(&self) -> Vec<EffectSpec> {
        let mut effects = self.payload.effect_specs(self.params.potency);
        if self.stun_seconds > 0.0 && self.payload.kind == ManifestationKind::Damage {
            effects.push(EffectSpec::ApplyStatus(ApplyStatusEffect {
                status_id: StatusId::new("stun"),
                duration_override_seconds: Some(self.stun_seconds),
                potency: 1.0,
            }));
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flame_payload_is_damage_plus_burn() {
        let specs = ManifestationPayload::damage(["burn"]).effect_specs(165.0);
        assert!(matches!(
            specs[0],
            EffectSpec::Damage(DamageEffect { amount }) if (amount - 165.0).abs() < f32::EPSILON
        ));
        assert!(matches!(
            &specs[1],
            EffectSpec::ApplyStatus(effect) if effect.status_id.as_str() == "burn"
        ));
        assert_eq!(specs.len(), 2);
    }

    #[test]
    fn life_payload_is_heal_without_burn() {
        let specs = ManifestationPayload::heal([]).effect_specs(165.0);
        assert!(matches!(specs[0], EffectSpec::Heal(_)));
        assert_eq!(specs.len(), 1);
    }

    #[test]
    fn frost_payload_is_damage_plus_slow() {
        let specs = ManifestationPayload::damage(["slow"]).effect_specs(100.0);
        assert!(matches!(specs[0], EffectSpec::Damage(_)));
        assert!(matches!(
            &specs[1],
            EffectSpec::ApplyStatus(effect) if effect.status_id.as_str() == "slow"
        ));
    }

    #[test]
    fn heal_does_not_keep_the_gesture_stun() {
        let blueprint = AbilityBlueprint {
            ability_id: AbilityId::new("test"),
            tags: vec![],
            geometry: AbilityGeometry::Circle { radius: 4.0 },
            cast_mode: AbilityCastMode::Instant,
            execution: BlueprintExecution::Base,
            params: AbilityParams {
                potency: 50.0,
                area: 4.0,
                range: 0.0,
                cast_time: 0.0,
                cooldown: 1.0,
                energy_cost: 0.0,
            },
            animation: "a",
            impact_vfx: "v",
            impact_delay: 0.0,
            stun_seconds: 0.8,
            payload: ManifestationPayload::heal([]),
        };
        let specs = blueprint.payload_effects();
        assert_eq!(specs.len(), 1);
        assert!(matches!(specs[0], EffectSpec::Heal(_)));
    }
}
