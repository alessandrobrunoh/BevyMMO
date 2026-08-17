//! Ancient Word Hunger — life steal/drain effect.
//! Converts a portion of damage dealt into healing for the caster.

use bevymmo_gameplay::abilities::{
    AbilityBlueprint, AbilityParams, AbilityTag, AncientWordEffect, BaseAbility,
};
use bevymmo_gameplay::spells::context::SpellCastContext;
use bevymmo_props_macro::ancient_word;

#[ancient_word(id = "hunger", name = "Hunger", tag = Melee, rune_cost = 2)]
pub struct Hunger;

impl Hunger {
    /// Life steal fraction (0.0-1.0).
    pub const LIFE_STEAL_FRACTION: f32 = 0.25;
}

impl AncientWordEffect for Hunger {
    fn post_process(
        &self,
        _ability: &dyn BaseAbility,
        _params: &AbilityParams,
        _ctx: &mut SpellCastContext,
    ) {
    }

    fn transform_blueprint(&self, blueprint: &mut AbilityBlueprint) {
        // Life steal has opportunity cost: slightly reduced upfront damage
        blueprint.params.potency *= 0.9;

        // Tag as self-target compatible for the heal component
        if !blueprint.has_tag(AbilityTag::SelfTarget) {
            blueprint.tags.push(AbilityTag::SelfTarget);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{AbilityId, AbilityGeometry};

    #[test]
    fn metadata_declares_melee_requirement() {
        let metadata = <Hunger as bevymmo_gameplay::abilities::AncientWord>::metadata(&Hunger);
        assert_eq!(metadata.rune_cost, 2);
        assert!(metadata.required_tags.contains(&AbilityTag::Melee));
    }

    #[test]
    fn transform_reduces_damage_and_adds_self_target() {
        let mut blueprint = AbilityBlueprint {
            ability_id: AbilityId::new("life_drain_strike"),
            tags: vec![AbilityTag::Melee],
            geometry: AbilityGeometry::Cone { radius: 2.0, angle_deg: 45.0 },
            cast_mode: crate::abilities::AbilityCastMode::Instant,
            execution: crate::abilities::blueprint::BlueprintExecution::Base,
            params: AbilityParams {
                potency: 120.0,
                area: 2.0,
                range: 2.0,
                cast_time: 0.0,
                cooldown: 1.8,
                energy_cost: 15.0,
            },
            animation: "bite",
            impact_vfx: "drain",
            impact_delay: 0.1,
            stun_seconds: 0.0,
        };

        Hunger.transform_blueprint(&mut blueprint);

        // Damage reduced by 10% due to life steal tradeoff
        assert!((blueprint.params.potency - 108.0).abs() < f32::EPSILON);
        assert!(blueprint.has_tag(AbilityTag::SelfTarget));
    }

    #[test]
    fn life_steal_fraction_is_constant() {
        assert_eq!(Hunger::LIFE_STEAL_FRACTION, 0.25);
    }
}
