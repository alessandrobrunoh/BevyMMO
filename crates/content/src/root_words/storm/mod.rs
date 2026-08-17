//! Root Word Storm — lightning/electric damage with chain potential.
//! Applies storm tag for chain lightning behavior.

use bevymmo_props_macro::root_word;

use crate::abilities::{AbilityBlueprint, AbilityParams, RootWordEffect, RootWordRegistry};

#[root_word(
    id = "storm",
    name = "Storm",
    description = "Applies lightning damage with chaining effects",
    rune_cost = 1
)]
pub struct StormRootWord;

/// Adds this content package to the root word registry.
pub fn register(registry: &mut RootWordRegistry) {
    StormRootWord::register(registry);
}

impl StormRootWord {
    /// Chain multiplier for storm abilities (bonus per target).
    pub const CHAIN_BONUS: f32 = 0.8;
}

impl RootWordEffect for StormRootWord {
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, _params: &AbilityParams) {
        // Tag as ranged projectile storm damage
        blueprint.tags.push(crate::abilities::AbilityTag::Ranged);
        blueprint.tags.push(crate::abilities::AbilityTag::Projectile);

        // Storm has high burst but single-target focused
        blueprint.params.potency *= 1.25;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::RootWord;

    #[test]
    fn id_is_stable() {
        assert_eq!(StormRootWord::ID, "storm");
    }

    #[test]
    fn metadata_values() {
        let word = StormRootWord;
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Storm");
        assert_eq!(meta.description, "Applies lightning damage with chaining effects");
        assert_eq!(meta.rune_cost, 1);
    }

    #[test]
    fn apply_to_blueprint_increases_burst_potency() {
        let word = StormRootWord;
        let mut blueprint = AbilityBlueprint {
            ability_id: crate::abilities::AbilityId::new("test"),
            tags: vec![],
            geometry: crate::abilities::AbilityGeometry::Projectile { speed: 30.0 },
            cast_mode: crate::abilities::AbilityCastMode::Instant,
            execution: crate::abilities::blueprint::BlueprintExecution::Base,
            params: crate::abilities::AbilityParams {
                potency: 80.0,
                area: 0.0,
                range: 25.0,
                cast_time: 0.0,
                cooldown: 1.5,
                energy_cost: 12.0,
            },
            animation: "cast",
            impact_vfx: "lightning_impact",
            impact_delay: 0.1,
            stun_seconds: 0.0,
        };
        let params = crate::abilities::AbilityParams {
            potency: 80.0,
            area: 0.0,
            range: 25.0,
            cast_time: 0.0,
            cooldown: 1.5,
            energy_cost: 12.0,
        };

        crate::abilities::RootWordEffect::apply_to_blueprint(&word, &mut blueprint, &params);

        assert!((blueprint.params.potency - 100.0).abs() < f32::EPSILON);
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::Ranged));
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::Projectile));
    }
}
