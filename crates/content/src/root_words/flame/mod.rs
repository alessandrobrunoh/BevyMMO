//! Root Word Flame — fire damage with burning potential.
//! Applies fire tag and increases potency scaling for damage-over-time effects.

use bevymmo_props_macro::root_word;

use crate::abilities::{AbilityBlueprint, AbilityParams, RootWordEffect, RootWordRegistry};

#[root_word(
    id = "flame",
    name = "Flame",
    description = "Applies fire damage with burning effects",
    rune_cost = 1
)]
pub struct FlameRootWord;

/// Adds this content package to the root word registry.
pub fn register(registry: &mut RootWordRegistry) {
    FlameRootWord::register(registry);
}

impl FlameRootWord {
    /// Potency multiplier for flame abilities.
    pub const FLAME_SCALING: f32 = 1.15;
}

impl RootWordEffect for FlameRootWord {
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, _params: &AbilityParams) {
        // Tag as ranged fire damage
        blueprint.tags.push(crate::abilities::AbilityTag::Ranged);
        blueprint.tags.push(crate::abilities::AbilityTag::Projectile);

        // Increase potency for fire scaling
        blueprint.params.potency *= Self::FLAME_SCALING;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::RootWord;

    #[test]
    fn id_is_stable() {
        assert_eq!(FlameRootWord::ID, "flame");
    }

    #[test]
    fn metadata_values() {
        let word = FlameRootWord;
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Flame");
        assert_eq!(meta.description, "Applies fire damage with burning effects");
        assert_eq!(meta.rune_cost, 1);
    }

    #[test]
    fn apply_to_blueprint_increases_potency() {
        let word = FlameRootWord;
        let mut blueprint = AbilityBlueprint {
            ability_id: crate::abilities::AbilityId::new("test"),
            tags: vec![],
            geometry: crate::abilities::AbilityGeometry::Projectile { speed: 10.0 },
            cast_mode: crate::abilities::AbilityCastMode::Instant,
            execution: crate::abilities::blueprint::BlueprintExecution::Base,
            params: crate::abilities::AbilityParams {
                potency: 100.0,
                area: 0.0,
                range: 20.0,
                cast_time: 0.0,
                cooldown: 1.0,
                energy_cost: 10.0,
            },
            animation: "cast",
            impact_vfx: "fire_impact",
            impact_delay: 0.5,
            stun_seconds: 0.0,
        };
        let params = crate::abilities::AbilityParams {
            potency: 100.0,
            area: 0.0,
            range: 20.0,
            cast_time: 0.0,
            cooldown: 1.0,
            energy_cost: 10.0,
        };

        crate::abilities::RootWordEffect::apply_to_blueprint(&word, &mut blueprint, &params);

        assert!((blueprint.params.potency - 115.0).abs() < f32::EPSILON);
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::Ranged));
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::Projectile));
    }
}
