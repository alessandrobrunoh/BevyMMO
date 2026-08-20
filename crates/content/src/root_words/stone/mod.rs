//! Root Word Stone — earth/physical damage with armor interaction.
//! Applies stone tag for physical damage and potential armor effects.

use bevymmo_props_macro::root_word;

use crate::abilities::{
    AbilityBlueprint, AbilityParams, ManifestationPayload, RootWordEffect, RootWordRegistry,
};

#[root_word(
    id = "stone",
    name = "Stone",
    description = "Applies physical earth damage with armor effects",
    rune_cost = 1
)]
pub struct StoneRootWord;

/// Adds this content package to the root word registry.
pub fn register(registry: &mut RootWordRegistry) {
    StoneRootWord::register(registry);
}

impl StoneRootWord {
    /// Armor interaction bonus for stone abilities.
    pub const ARMOR_BONUS: f32 = 1.0;
}

impl RootWordEffect for StoneRootWord {
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, _params: &AbilityParams) {
        // Tag as melee physical damage
        blueprint.tags.push(crate::abilities::AbilityTag::Melee);

        // Stone is consistent but not flashy
        blueprint.params.potency *= 1.05;
        blueprint.payload = ManifestationPayload::damage([]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::RootWord;

    #[test]
    fn id_is_stable() {
        assert_eq!(StoneRootWord::ID, "stone");
    }

    #[test]
    fn metadata_values() {
        let word = StoneRootWord;
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Stone");
        assert_eq!(
            meta.description,
            "Applies physical earth damage with armor effects"
        );
        assert_eq!(meta.rune_cost, 1);
    }

    #[test]
    fn apply_to_blueprint_applies_melee_tags() {
        let word = StoneRootWord;
        let mut blueprint = AbilityBlueprint {
            ability_id: crate::abilities::AbilityId::new("test"),
            tags: vec![],
            geometry: crate::abilities::AbilityGeometry::Cone {
                radius: 3.0,
                angle_deg: 90.0,
            },
            cast_mode: crate::abilities::AbilityCastMode::Instant,
            execution: crate::abilities::blueprint::BlueprintExecution::Base,
            params: crate::abilities::AbilityParams {
                potency: 70.0,
                area: 3.0,
                range: 3.0,
                cast_time: 0.0,
                cooldown: 1.8,
                energy_cost: 14.0,
            },
            animation: "slam",
            impact_vfx: "rock_impact",
            impact_delay: 0.2,
            stun_seconds: 0.0,
            payload: ManifestationPayload::default(),
        };
        let params = crate::abilities::AbilityParams {
            potency: 70.0,
            area: 3.0,
            range: 3.0,
            cast_time: 0.0,
            cooldown: 1.8,
            energy_cost: 14.0,
        };

        crate::abilities::RootWordEffect::apply_to_blueprint(&word, &mut blueprint, &params);

        assert!((blueprint.params.potency - 73.5).abs() < f32::EPSILON);
        assert!(blueprint.has_tag(crate::abilities::AbilityTag::Melee));
    }
}
