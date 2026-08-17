//! Amplia — aumenta la geometria ad area con un costo di Potency.

use bevymmo_gameplay::abilities::{
    AbilityBlueprint, AbilityGeometry, AbilityParams, AbilityTag, AncientWordEffect, BaseAbility,
};
use bevymmo_gameplay::spells::context::SpellCastContext;
use bevymmo_props_macro::ancient_word;

#[ancient_word(id = "amplia", name = "Amplia", tag = Area, rune_cost = 2)]
pub struct Amplia;

impl Amplia {
    pub const AREA_MULTIPLIER: f32 = 1.35;
    pub const POTENCY_MULTIPLIER: f32 = 0.90;
}

impl AncientWordEffect for Amplia {
    fn post_process(
        &self,
        _ability: &dyn BaseAbility,
        _params: &AbilityParams,
        _ctx: &mut SpellCastContext,
    ) {
    }

    fn transform_blueprint(&self, blueprint: &mut AbilityBlueprint) {
        blueprint.params.potency *= Self::POTENCY_MULTIPLIER;
        blueprint.geometry = match blueprint.geometry {
            AbilityGeometry::Circle { radius } => AbilityGeometry::Circle {
                radius: radius * Self::AREA_MULTIPLIER,
            },
            geometry => geometry,
        };
        blueprint.tags.push(AbilityTag::Area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::AbilityId;

    #[test]
    fn metadata_declares_area_compatibility_and_conflict_group() {
        let metadata = <Amplia as bevymmo_gameplay::abilities::AncientWord>::metadata(&Amplia);
        assert_eq!(metadata.rune_cost, 2);
        assert_eq!(metadata.required_tags, vec![AbilityTag::Area]);
    }

    #[test]
    fn transform_reduces_potency_and_expands_circles() {
        let ability = crate::ability_definitions::arcane_orb::ArcaneOrb;
        let mut blueprint = ability.blueprint();
        blueprint.geometry = AbilityGeometry::Circle { radius: 4.0 };
        blueprint.params.potency = 100.0;
        Amplia.transform_blueprint(&mut blueprint);

        assert_eq!(blueprint.ability_id, AbilityId::new("arcane_orb"));
        assert!((blueprint.params.potency - 90.0).abs() < f32::EPSILON);
        assert!(
            (matches!(blueprint.geometry, AbilityGeometry::Circle { radius } if (radius - 5.4).abs() < f32::EPSILON))
        );
    }
}
