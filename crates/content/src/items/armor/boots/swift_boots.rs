//! Swift Boots — lightweight boots that enhance movement.

use bevymmo_props_macro::item;

use crate::items::ItemRegistry;

#[item(
    id = "swift_boots",
    name = "Swift Boots",
    description = "Boots crafted from supple leather and enchanted for speed. Every step feels lighter.",
    category = Armor,
    rarity = Uncommon,
    slot = Shoes,
    tradable = true,
    effects = [
        stat_bonus(field = Armor, op = Add, value = 5.0),
        stat_bonus(field = Speed, op = Add, value = 0.1),
    ],
    rune_profile(capacity = 5, stability = 0.94),
)]
pub struct SwiftBoots;

pub fn register(registry: &mut ItemRegistry) {
    SwiftBoots::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::components::EquipSlot;
    use crate::items::definition::Item;

    #[test]
    fn id_and_slot_match_design() {
        let item = SwiftBoots;
        assert_eq!(item.id().as_str(), "swift_boots");
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Shoes));
    }

    #[test]
    fn category_is_armor() {
        let item = SwiftBoots;
        assert!(matches!(
            item.config().category,
            crate::items::ItemCategory::Armor
        ));
    }

    #[test]
    fn has_no_ability_loadout() {
        assert!(SwiftBoots.ability_loadout().is_none());
    }

    #[test]
    fn has_a_rune_profile_with_stability() {
        let item = SwiftBoots;
        let profile = item
            .rune_profile()
            .expect("swift_boots must grant a rune profile");
        assert_eq!(profile.capacity, 5);
        assert!((profile.stability - 0.94).abs() < f32::EPSILON);
    }

    #[test]
    fn grants_armor_and_speed_bonuses() {
        let item = SwiftBoots;
        assert_eq!(item.effects().len(), 2);
    }
}
