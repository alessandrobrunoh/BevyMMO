//! Root Word Life — healing and restoration.
//! Converts damage into healing and applies restoration tags.

use bevymmo_props_macro::root_word;

use crate::abilities::{AbilityBlueprint, AbilityParams, RootWordEffect, RootWordRegistry};

#[root_word(
    id = "life",
    name = "Life",
    description = "Restores health to allies",
    rune_cost = 1
)]
pub struct LifeRootWord;

/// Adds this content package to the root word registry.
pub fn register(registry: &mut RootWordRegistry) {
    LifeRootWord::register(registry);
}

impl LifeRootWord {
    /// Healing efficiency multiplier.
    pub const HEALING_EFFICIENCY: f32 = 1.2;
}

impl RootWordEffect for LifeRootWord {
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, _params: &AbilityParams) {
        // Tag as self-buff or friendly target healing
        blueprint.tags.push(crate::abilities::AbilityTag::SelfTarget);

        // Healing is more potent than equivalent damage
        blueprint.params.potency *= Self::HEALING_EFFICIENCY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::RootWord;

    #[test]
    fn id_is_stable() {
        assert_eq!(LifeRootWord::ID, "life");
    }

    #[test]
    fn metadata_values() {
        let word = LifeRootWord;
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Life");
        assert_eq!(meta.description, "Restores health to allies");
        assert_eq!(meta.rune_cost, 1);
    }

    #[test]
    fn apply_to_blueprint_converts_to_healing() {
        let word = LifeRootWord;
        let mut blueprint = AbilityBlueprint {
            ability_id: crate::abilities::AbilityId::new("test"),
            tags: vec![],
            geometry: crate::abilities::AbilityGeometry::SelfBuff { duration_seconds: 3.0 },
            cast_mode: crate::abilities::AbilityCastMode::Instant,
            execution: crate::abilities::blueprint::BlueprintExecution::Base,
            params: crate::abilities::AbilityParams {
                potency: 50.0,
                area: 0.0,
                range: 0.0,
                cast_time: 0.0,
                cooldown: 3.0,
                energy_cost: 20.0,
            },
            animation: "heal",
            impact_vfx: "heal_effect",
            impact_delay: 0.0,
            stun_seconds: 0.0,
        };
        let params = crate::abilities::AbilityParams {
            potency: 50.0,
            area: 0.0,
            range: 0.0,
            cast_time: 0.0,
            cooldown: 3.0,
            energy_cost: 20.0,
        };

        crate::abilities::RootWordEffect::apply_to_blueprint(&word, &mut blueprint, &params);

        // Life increases potency by 20% (healing efficiency): 50 * 1.2 = 60
        assert!((blueprint.params.potency - 60.0).abs() < 0.001);
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::SelfTarget));
    }
}
