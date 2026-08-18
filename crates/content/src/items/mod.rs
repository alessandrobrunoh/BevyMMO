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
    weapons::staff::mage_staff::register(&mut registry);
    weapons::bow::bow::register(&mut registry);
    weapons::sword::sword::register(&mut registry);
    weapons::hammer::hammer::register(&mut registry);

    registry
}

/// Builds the registry containing every weapon family shipped by this game build.
pub fn default_weapon_families() -> WeaponFamilyRegistry {
    let mut registry = WeaponFamilyRegistry::default();
    weapons::staff::register(&mut registry);
    weapons::bow::register(&mut registry);
    weapons::sword::register(&mut registry);
    weapons::hammer::register(&mut registry);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::registry::ItemId;

    #[test]
    fn default_items_contains_all_items() {
        let registry = default_items();

        // Staff
        assert!(registry.contains(&ItemId::new(weapons::staff::mage_staff::MageStaff::ID)));
        // Armor items
        assert!(registry.contains(&ItemId::new(
            armor::chestplate::robust_cuirass::RobustCuirass::ID
        )));
        assert!(registry.contains(&ItemId::new(armor::helmet::warding_helm::WardingHelm::ID)));
        assert!(registry.contains(&ItemId::new(armor::boots::swift_boots::SwiftBoots::ID)));
        // Accessory items
        assert!(registry.contains(&ItemId::new(purity_charm::PurityCharm::ID)));

        // One weapon per family
        assert!(registry.contains(&ItemId::new(weapons::bow::bow::Bow::ID)));
        assert!(registry.contains(&ItemId::new(weapons::sword::sword::Sword::ID)));
        assert!(registry.contains(&ItemId::new(weapons::hammer::hammer::Hammer::ID)));
        assert!(registry.contains(&ItemId::new(weapons::staff::mage_staff::MageStaff::ID)));

        assert_eq!(registry.len(), 8); // 4 non-weapons + 4 weapons
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
        assert_eq!(registry.len(), 4);
    }
}
