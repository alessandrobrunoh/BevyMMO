//! Root Word Void — entropy/void damage that ignores resistance.
//! Applies void tag for true damage or resistance-piercing effects.

use bevymmo_props_macro::root_word;

use crate::abilities::{AbilityBlueprint, AbilityParams, RootWordEffect, RootWordRegistry};

#[root_word(
    id = "void",
    name = "Void",
    description = "Applies void damage that bypasses resistances",
    rune_cost = 2
)]
pub struct VoidRootWord;

/// Adds this content package to the root word registry.
pub fn register(registry: &mut RootWordRegistry) {
    VoidRootWord::register(registry);
}

impl VoidRootWord {
    /// Resistance penetration factor (0.0-1.0, portion of damage that ignores resistance).
    pub const RESISTANCE_PENETRATION: f32 = 0.5;
}

impl RootWordEffect for VoidRootWord {
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, _params: &AbilityParams) {
        // Tag as ranged void damage
        blueprint.tags.push(crate::abilities::AbilityTag::Ranged);

        // Void damage is premium but pierces defenses
        blueprint.params.potency *= 1.1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::RootWord;

    #[test]
    fn id_is_stable() {
        assert_eq!(VoidRootWord::ID, "void");
    }

    #[test]
    fn metadata_values() {
        let word = VoidRootWord;
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Void");
        assert_eq!(
            meta.description,
            "Applies void damage that bypasses resistances"
        );
        // Void costs more due to resistance penetration
        assert_eq!(meta.rune_cost, 2);
    }

    #[test]
    fn apply_to_blueprint_applies_void_scaling() {
        let word = VoidRootWord;
        let mut blueprint = AbilityBlueprint {
            ability_id: crate::abilities::AbilityId::new("test"),
            tags: vec![],
            geometry: crate::abilities::AbilityGeometry::Projectile { speed: 15.0 },
            cast_mode: crate::abilities::AbilityCastMode::Instant,
            execution: crate::abilities::blueprint::BlueprintExecution::Base,
            params: crate::abilities::AbilityParams {
                potency: 90.0,
                area: 0.0,
                range: 18.0,
                cast_time: 0.5,
                cooldown: 4.0,
                energy_cost: 25.0,
            },
            animation: "void_cast",
            impact_vfx: "void_impact",
            impact_delay: 0.7,
            stun_seconds: 0.0,
        };
        let params = crate::abilities::AbilityParams {
            potency: 90.0,
            area: 0.0,
            range: 18.0,
            cast_time: 0.5,
            cooldown: 4.0,
            energy_cost: 25.0,
        };

        crate::abilities::RootWordEffect::apply_to_blueprint(&word, &mut blueprint, &params);

        assert!((blueprint.params.potency - 99.0).abs() < f32::EPSILON);
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::Ranged));
    }

    #[test]
    fn void_has_higher_rune_cost() {
        let word = VoidRootWord;
        assert_eq!(word.metadata().rune_cost, 2);
    }
}
