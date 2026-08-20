//! Root Word content and its registry.

pub mod damage;
pub mod flame;
pub mod frost;
pub mod life;
pub mod stone;
pub mod storm;
pub mod void;

use crate::abilities::RootWordRegistry;

/// Builds the registry containing every root word shipped by this game build.
pub fn default_root_words() -> RootWordRegistry {
    let mut registry = RootWordRegistry::default();
    damage::register(&mut registry);
    flame::register(&mut registry);
    frost::register(&mut registry);
    storm::register(&mut registry);
    life::register(&mut registry);
    void::register(&mut registry);
    stone::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{
        cast_root_inscribed_slot, resolve_root_inscribed_slot, AbilitySelection, AbilitySlot,
        KnownAncientLanguage, ManifestationKind, ManifestationPayload, RootWordId, SlotPreview,
        WeaponInscription,
    };
    use crate::effects::{DamageEffect, EffectSpec, HealEffect};
    use crate::item_definitions::weapons::staff::mage_staff::MageStaff;
    use crate::items::Item;
    use crate::stats::components::CombatStats;
    use crate::EntityId;
    use glam::Vec3;

    fn know(root: &'static str) -> KnownAncientLanguage {
        let mut known = KnownAncientLanguage::default();
        known.root_words.insert(RootWordId::from(root));
        known
    }

    fn inscription(root: &'static str) -> WeaponInscription {
        WeaponInscription {
            root_word: Some(RootWordId::from(root)),
            ..Default::default()
        }
    }

    fn resolve_staff_slot(slot: AbilitySlot, root: &'static str) -> SlotPreview {
        let abilities = crate::ability_definitions::default_base_abilities();
        let roots = default_root_words();
        let words = crate::ancient_word_definitions::default_ancient_words();
        let staff = MageStaff;
        let loadout = staff
            .ability_loadout()
            .expect("mage staff offers a loadout");
        resolve_root_inscribed_slot(
            slot,
            loadout,
            &AbilitySelection::default(),
            &inscription(root),
            &know(root),
            &abilities,
            &roots,
            &words,
            Some(&staff),
        )
        .expect("inscribed staff slot should resolve")
    }

    fn cast_staff_primary(root: &'static str) -> Vec<EffectSpec> {
        let abilities = crate::ability_definitions::default_base_abilities();
        let roots = default_root_words();
        let words = crate::ancient_word_definitions::default_ancient_words();
        let staff = MageStaff;
        let loadout = staff
            .ability_loadout()
            .expect("mage staff offers a loadout");
        let combat = CombatStats {
            attack_power: 10.0,
            armor: 0.0,
        };
        let target = EntityId::new(2);
        let potential = [(target, Vec3::new(0.0, 0.0, 8.0))];
        let mut ctx = crate::spells::context::SpellCastContext::new(
            EntityId::new(1),
            Vec3::ZERO,
            &combat,
            Vec3::Z,
            None,
            Some(target),
            &potential,
        );
        cast_root_inscribed_slot(
            AbilitySlot::Primary,
            loadout,
            &AbilitySelection::default(),
            &inscription(root),
            &know(root),
            &abilities,
            &roots,
            &words,
            &mut ctx,
            Some(&staff),
        )
        .expect("inscribed staff primary should cast");
        ctx.pending_projectiles
            .pop()
            .expect("arcane bolt emits a projectile")
            .effects
    }

    #[test]
    fn arcane_bolt_flame_is_damage_plus_burn() {
        let preview = resolve_staff_slot(AbilitySlot::Primary, "flame");
        assert_eq!(preview.ability.id().as_str(), "arcane_bolt");
        assert_eq!(
            preview.blueprint.payload,
            ManifestationPayload::damage(["burn"])
        );
        let effects = cast_staff_primary("flame");
        assert!(matches!(
            effects[0],
            EffectSpec::Damage(DamageEffect { amount }) if (amount - 165.0 * 1.15).abs() < 0.001
        ));
        assert!(matches!(
            &effects[1],
            EffectSpec::ApplyStatus(effect) if effect.status_id.as_str() == "burn"
        ));
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn arcane_bolt_life_is_heal_without_burn() {
        let preview = resolve_staff_slot(AbilitySlot::Primary, "life");
        assert_eq!(preview.blueprint.payload.kind, ManifestationKind::Heal);
        assert!(preview.blueprint.payload.status_ids.is_empty());
        let effects = cast_staff_primary("life");
        assert!(
            matches!(effects[0], EffectSpec::Heal(HealEffect { amount }) if (amount - 165.0 * 1.2).abs() < 0.001)
        );
        assert_eq!(effects.len(), 1);
    }

    #[test]
    fn arcane_bolt_frost_is_damage_plus_slow() {
        let preview = resolve_staff_slot(AbilitySlot::Primary, "frost");
        assert_eq!(
            preview.blueprint.payload,
            ManifestationPayload::damage(["slow"])
        );
        let effects = cast_staff_primary("frost");
        assert!(matches!(effects[0], EffectSpec::Damage(_)));
        assert!(matches!(
            &effects[1],
            EffectSpec::ApplyStatus(effect) if effect.status_id.as_str() == "slow"
        ));
        assert!(!effects.iter().any(|spec| matches!(
            spec,
            EffectSpec::ApplyStatus(effect) if effect.status_id.as_str() == "burn"
        )));
    }

    #[test]
    fn staff_qwe_share_the_same_flame_payload() {
        for slot in AbilitySlot::ALL {
            let preview = resolve_staff_slot(slot, "flame");
            assert_eq!(
                preview.blueprint.payload,
                ManifestationPayload::damage(["burn"]),
                "slot {slot:?} should follow Flame, not the gesture"
            );
        }
    }

    #[test]
    fn default_root_words_contains_all_seven() {
        let reg = default_root_words();
        assert_eq!(reg.len(), 7);
        assert!(reg.contains(&crate::abilities::RootWordId::from("damage")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("flame")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("frost")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("storm")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("life")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("void")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("stone")));
    }

    #[test]
    fn damage_metadata_preserved() {
        let reg = default_root_words();
        let word = reg
            .get(&crate::abilities::RootWordId::from("damage"))
            .unwrap();
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Danno");
        assert_eq!(meta.rune_cost, 1);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn all_root_words_have_stable_ids() {
        let reg = default_root_words();
        let ids = ["damage", "flame", "frost", "storm", "life", "void", "stone"];
        for id in ids.iter() {
            assert!(
                reg.contains(&crate::abilities::RootWordId::from(*id)),
                "Missing root word: {id}"
            );
        }
    }

    #[test]
    fn void_has_higher_rune_cost() {
        let reg = default_root_words();
        let word = reg
            .get(&crate::abilities::RootWordId::from("void"))
            .unwrap();
        assert_eq!(word.metadata().rune_cost, 2);
    }
}
