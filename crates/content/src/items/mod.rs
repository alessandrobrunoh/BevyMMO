//! Item content and its registry.

pub mod armor;

pub mod purity_charm;
pub mod weapons;

use crate::items::registry::ItemRegistry;
use crate::items::WeaponFamilyRegistry;

/// Builds the registry containing every item shipped by this game build.
pub fn default_items() -> ItemRegistry {
    let mut registry = ItemRegistry::default();

    // Existing items
    armor::register(&mut registry);

    purity_charm::register(&mut registry);
    weapons::staff::conduit_staff_t4::register(&mut registry);
    weapons::staff::echo_staff::register(&mut registry);

    // Bow family
    weapons::bow::longbow::register(&mut registry);
    weapons::bow::swiftbow::register(&mut registry);

    // Sword family
    weapons::sword::arming_sword::register(&mut registry);
    weapons::sword::greatsword::register(&mut registry);

    // Hammer family
    weapons::hammer::warhammer::register(&mut registry);
    weapons::hammer::maul::register(&mut registry);

    // Focus family
    weapons::focus::arcane_focus::register(&mut registry);
    weapons::focus::primal_focus::register(&mut registry);

    // Gauntlets family
    weapons::gauntlets::battle_gauntlets::register(&mut registry);
    weapons::gauntlets::iron_fists::register(&mut registry);

    registry
}

/// Builds the registry containing every weapon family shipped by this game build.
pub fn default_weapon_families() -> WeaponFamilyRegistry {
    let mut registry = WeaponFamilyRegistry::default();
    weapons::staff::register(&mut registry);
    weapons::bow::register(&mut registry);
    weapons::sword::register(&mut registry);
    weapons::hammer::register(&mut registry);
    weapons::focus::register(&mut registry);
    weapons::gauntlets::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::registry::ItemId;

    #[test]
    fn default_items_contains_all_items() {
        let registry = default_items();

        // Staff variants
        assert!(registry.contains(&ItemId::new(
            weapons::staff::conduit_staff_t4::ConduitStaffT4::ID
        )));
        assert!(registry.contains(&ItemId::new(weapons::staff::echo_staff::EchoStaff::ID)));
        // Armor items
        assert!(registry.contains(&ItemId::new(
            armor::chestplate::robust_cuirass::RobustCuirass::ID
        )));
        assert!(registry.contains(&ItemId::new(armor::helmet::warding_helm::WardingHelm::ID)));
        assert!(registry.contains(&ItemId::new(armor::boots::swift_boots::SwiftBoots::ID)));
        // Accessory items
        assert!(registry.contains(&ItemId::new(purity_charm::PurityCharm::ID)));

        // New weapon items (12)
        // Bow family
        assert!(registry.contains(&ItemId::new(weapons::bow::longbow::Longbow::ID)));
        assert!(registry.contains(&ItemId::new(weapons::bow::swiftbow::Swiftbow::ID)));
        // Sword family
        assert!(registry.contains(&ItemId::new(weapons::sword::arming_sword::ArmingSword::ID)));
        assert!(registry.contains(&ItemId::new(weapons::sword::greatsword::Greatsword::ID)));
        // Hammer family
        assert!(registry.contains(&ItemId::new(weapons::hammer::warhammer::Warhammer::ID)));
        assert!(registry.contains(&ItemId::new(weapons::hammer::maul::Maul::ID)));
        // Focus family
        assert!(registry.contains(&ItemId::new(weapons::focus::arcane_focus::ArcaneFocus::ID)));
        assert!(registry.contains(&ItemId::new(weapons::focus::primal_focus::PrimalFocus::ID)));
        // Gauntlets family
        assert!(registry.contains(&ItemId::new(weapons::gauntlets::battle_gauntlets::BattleGauntlets::ID)));
        assert!(registry.contains(&ItemId::new(weapons::gauntlets::iron_fists::IronFists::ID)));

        assert_eq!(registry.len(), 16); // 4 non-weapons + 12 weapon variants
    }

    #[test]
    fn default_items_armor_slots_are_correct() {
        use crate::items::components::EquipSlot;
        let registry = default_items();

        let cuirass = registry
            .get(&ItemId::new(
                armor::chestplate::robust_cuirass::RobustCuirass::ID,
            ))
            .unwrap();
        assert_eq!(cuirass.config().equippable_into, Some(EquipSlot::Armor));

        let helm = registry
            .get(&ItemId::new(armor::helmet::warding_helm::WardingHelm::ID))
            .unwrap();
        assert_eq!(helm.config().equippable_into, Some(EquipSlot::Helmet));

        let boots = registry
            .get(&ItemId::new(armor::boots::swift_boots::SwiftBoots::ID))
            .unwrap();
        assert_eq!(boots.config().equippable_into, Some(EquipSlot::Shoes));
    }

    #[test]
    fn default_weapon_families_contains_all_families() {
        let registry = default_weapon_families();
        assert!(registry.contains(&crate::items::WeaponFamilyId::new("staff")));
        assert!(registry.contains(&crate::items::WeaponFamilyId::new("bow")));
        assert!(registry.contains(&crate::items::WeaponFamilyId::new("sword")));
        assert!(registry.contains(&crate::items::WeaponFamilyId::new("hammer")));
        assert!(registry.contains(&crate::items::WeaponFamilyId::new("focus")));
        assert!(registry.contains(&crate::items::WeaponFamilyId::new("gauntlets")));
        assert_eq!(registry.len(), 6); // 1 original + 5 new families
    }
}
