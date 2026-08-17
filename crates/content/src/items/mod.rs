//! Item content and its registry.

pub mod armor;
pub mod magic_staff;
pub mod purity_charm;
pub mod weapons;

use crate::items::registry::ItemRegistry;
use crate::items::WeaponFamilyRegistry;

/// Builds the registry containing every item shipped by this game build.
pub fn default_items() -> ItemRegistry {
    let mut registry = ItemRegistry::default();
    armor::register(&mut registry);
    magic_staff::register(&mut registry);
    purity_charm::register(&mut registry);
    weapons::staff::conduit_staff_t4::register(&mut registry);
    weapons::staff::echo_staff::register(&mut registry);
    registry
}

/// Builds the registry containing every weapon family shipped by this game build.
pub fn default_weapon_families() -> WeaponFamilyRegistry {
    let mut registry = WeaponFamilyRegistry::default();
    weapons::staff::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::registry::ItemId;

    #[test]
    fn default_items_contains_staff_items() {
        let registry = default_items();
        assert!(registry.contains(&ItemId::new(magic_staff::MagicStaff::ID)));
        assert!(registry.contains(&ItemId::new(weapons::staff::conduit_staff_t4::ConduitStaffT4::ID)));
        assert!(registry.contains(&ItemId::new(weapons::staff::echo_staff::EchoStaff::ID)));
        // Armor items
        assert!(registry.contains(&ItemId::new(armor::chestplate::robust_cuirass::RobustCuirass::ID)));
        assert!(registry.contains(&ItemId::new(armor::helmet::warding_helm::WardingHelm::ID)));
        assert!(registry.contains(&ItemId::new(armor::boots::swift_boots::SwiftBoots::ID)));
        // Accessory items
        assert!(registry.contains(&ItemId::new(purity_charm::PurityCharm::ID)));
        assert_eq!(registry.len(), 7); // 3 weapons + 3 armor + 1 accessory
    }

    #[test]
    fn default_items_armor_slots_are_correct() {
        use crate::items::components::EquipSlot;
        let registry = default_items();

        let cuirass = registry.get(&ItemId::new(armor::chestplate::robust_cuirass::RobustCuirass::ID)).unwrap();
        assert_eq!(cuirass.config().equippable_into, Some(EquipSlot::Armor));

        let helm = registry.get(&ItemId::new(armor::helmet::warding_helm::WardingHelm::ID)).unwrap();
        assert_eq!(helm.config().equippable_into, Some(EquipSlot::Helmet));

        let boots = registry.get(&ItemId::new(armor::boots::swift_boots::SwiftBoots::ID)).unwrap();
        assert_eq!(boots.config().equippable_into, Some(EquipSlot::Shoes));
    }

    #[test]
    fn default_weapon_families_contains_staff() {
        let registry = default_weapon_families();
        assert!(registry.contains(&crate::items::WeaponFamilyId::new("staff")));
        assert_eq!(registry.len(), 1);
    }
}
