//! Root Word Frost — ice damage with slowing potential.
//! Applies frost tag and reduces target movement through chill effects.

use bevymmo_props_macro::root_word;

use crate::abilities::{AbilityBlueprint, AbilityParams, RootWordEffect, RootWordRegistry};

#[root_word(
    id = "frost",
    name = "Frost",
    description = "Applies ice damage with slowing effects",
    rune_cost = 1
)]
pub struct FrostRootWord;

/// Adds this content package to the root word registry.
pub fn register(registry: &mut RootWordRegistry) {
    FrostRootWord::register(registry);
}

impl FrostRootWord {
    /// Slow factor applied by frost abilities (as penalty).
    pub const SLOW_FACTOR: f32 = 0.85;
}

impl RootWordEffect for FrostRootWord {
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, _params: &AbilityParams) {
        // Tag as area frost damage
        blueprint.tags.push(crate::abilities::AbilityTag::Area);
        blueprint.tags.push(crate::abilities::AbilityTag::Ground);

        // Frost abilities have slightly reduced base potency but control value
        blueprint.params.potency *= 0.9;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::RootWord;

    #[test]
    fn id_is_stable() {
        assert_eq!(FrostRootWord::ID, "frost");
    }

    #[test]
    fn metadata_values() {
        let word = FrostRootWord;
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Frost");
        assert_eq!(meta.description, "Applies ice damage with slowing effects");
        assert_eq!(meta.rune_cost, 1);
    }

    #[test]
    fn apply_to_blueprint_reduces_potency_and_adds_area_tags() {
        let word = FrostRootWord;
        let mut blueprint = AbilityBlueprint {
            ability_id: crate::abilities::AbilityId::new("test"),
            tags: vec![],
            geometry: crate::abilities::AbilityGeometry::Circle { radius: 5.0 },
            cast_mode: crate::abilities::AbilityCastMode::Instant,
            execution: crate::abilities::blueprint::BlueprintExecution::Base,
            params: crate::abilities::AbilityParams {
                potency: 100.0,
                area: 5.0,
                range: 10.0,
                cast_time: 0.0,
                cooldown: 2.0,
                energy_cost: 15.0,
            },
            animation: "cast",
            impact_vfx: "frost_impact",
            impact_delay: 0.3,
            stun_seconds: 0.0,
        };
        let params = crate::abilities::AbilityParams {
            potency: 100.0,
            area: 5.0,
            range: 10.0,
            cast_time: 0.0,
            cooldown: 2.0,
            energy_cost: 15.0,
        };

        crate::abilities::RootWordEffect::apply_to_blueprint(&word, &mut blueprint, &params);

        assert!((blueprint.params.potency - 90.0).abs() < f32::EPSILON);
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::Area));
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::Ground));
    }
}
