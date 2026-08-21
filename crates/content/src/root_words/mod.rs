//! Root Word content and its registry.

pub mod flame;
pub mod life;

use crate::abilities::RootWordRegistry;

/// Builds the registry containing every root word shipped by this game build.
pub fn default_root_words() -> RootWordRegistry {
    let mut registry = RootWordRegistry::default();
    flame::register(&mut registry);
    life::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{
        cast_root_inscribed_slot, resolve_root_inscribed_slot, AbilitySelection, AbilitySlot,
        AbilityTag, KnownAncientLanguage, ManifestationKind, ManifestationPayload, RootWordId,
        SlotPreview, WeaponInscription,
    };
    use crate::effects::{DamageEffect, EffectSpec, HealEffect};
    use crate::item_definitions::weapons::sword::sword::Sword;
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

    fn resolve_sword_slot(slot: AbilitySlot, root: &'static str) -> SlotPreview {
        let abilities = crate::ability_definitions::default_base_abilities();
        let roots = default_root_words();
        let words = crate::ancient_word_definitions::default_ancient_words();
        let sword = Sword;
        let loadout = sword.ability_loadout().expect("sword offers a loadout");
        resolve_root_inscribed_slot(
            slot,
            loadout,
            &AbilitySelection::default(),
            &inscription(root),
            &know(root),
            &abilities,
            &roots,
            &words,
            Some(&sword),
        )
        .expect("inscribed sword slot should resolve")
    }

    fn cast_sword_primary(root: &'static str) -> Vec<EffectSpec> {
        let abilities = crate::ability_definitions::default_base_abilities();
        let roots = default_root_words();
        let words = crate::ancient_word_definitions::default_ancient_words();
        let sword = Sword;
        let loadout = sword.ability_loadout().expect("sword offers a loadout");
        let combat = CombatStats {
            attack_power: 10.0,
            armor: 0.0,
        };
        let target = EntityId::new(2);
        let potential = [(target, Vec3::new(0.0, 0.0, 2.0))];
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
            Some(&sword),
        )
        .expect("inscribed sword primary should cast");
        ctx.pending_aoes
            .pop()
            .expect("cleave emits an area effect")
            .effects
    }

    #[test]
    fn cleave_flame_is_damage_plus_burn() {
        let preview = resolve_sword_slot(AbilitySlot::Primary, "flame");
        assert_eq!(preview.ability.id().as_str(), "cleave");
        assert_eq!(
            preview.blueprint.payload,
            ManifestationPayload::damage(["burn"])
        );
        assert!(preview.blueprint.has_tag(AbilityTag::Melee));
        assert!(preview.blueprint.has_tag(AbilityTag::Area));
        assert!(!preview.blueprint.has_tag(AbilityTag::Projectile));
        let effects = cast_sword_primary("flame");
        assert!(matches!(
            effects[0],
            EffectSpec::Damage(DamageEffect { amount }) if (amount - 115.0 * 1.15).abs() < 0.001
        ));
        assert!(matches!(
            &effects[1],
            EffectSpec::ApplyStatus(effect) if effect.status_id.as_str() == "burn"
        ));
    }

    #[test]
    fn cleave_life_is_heal() {
        let preview = resolve_sword_slot(AbilitySlot::Primary, "life");
        assert_eq!(preview.blueprint.payload.kind, ManifestationKind::Heal);
        assert!(preview.blueprint.payload.status_ids.is_empty());
        let effects = cast_sword_primary("life");
        assert!(
            matches!(effects[0], EffectSpec::Heal(HealEffect { amount }) if (amount - 115.0 * 1.2).abs() < 0.001)
        );
    }

    #[test]
    fn sword_qwe_share_the_same_flame_payload() {
        for slot in AbilitySlot::ALL {
            let preview = resolve_sword_slot(slot, "flame");
            assert_eq!(
                preview.blueprint.payload,
                ManifestationPayload::damage(["burn"]),
                "slot {slot:?} should follow Flame, not the gesture"
            );
        }
    }

    #[test]
    fn default_root_words_contains_flame_and_life() {
        let reg = default_root_words();
        assert_eq!(reg.len(), 2);
        assert!(reg.contains(&crate::abilities::RootWordId::from("flame")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("life")));
        assert!(!reg.contains(&crate::abilities::RootWordId::from("frost")));
        assert!(!reg.contains(&crate::abilities::RootWordId::from("damage")));
    }
}
