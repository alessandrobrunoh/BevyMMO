//! Swift Boots — lightweight boots that enhance movement.

use bevymmo_props_macro::item;

use crate::ability_definitions::arcane_orb::ArcaneOrb;
use crate::items::ItemRegistry;

#[item(
    id = "swift_boots",
    name = "Swift Boots",
    description = "Boots crafted from supple leather and enchanted for speed. Every step feels lighter.",
    category = Armor,
    rarity = Uncommon,
    slot = Shoes,
    effects = [
        stat_bonus(field = Armor, op = Add, value = 5.0),
        stat_bonus(field = Speed, op = Add, value = 0.1),
    ],
    abilities(
        primary = [ArcaneOrb],
        secondary = [ArcaneOrb],
        ultimate = [ArcaneOrb],
    ),
    rune_profile(capacity = 5, stability = 0.94, affinity = fuoco),
)]
pub struct SwiftBoots;

pub fn register(registry: &mut ItemRegistry) {
    SwiftBoots::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilitySlot;
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
        assert!(matches!(item.config().category, crate::items::ItemCategory::Armor));
    }

    #[test]
    fn offers_arcane_orb_for_every_slot() {
        let item = SwiftBoots;
        let abilities = item
            .ability_loadout()
            .expect("swift_boots must grant abilities");
        let expected = [ArcaneOrb::ID.into()];

        assert_eq!(abilities.options_for(AbilitySlot::Primary), expected);
        assert_eq!(abilities.options_for(AbilitySlot::Secondary), expected);
        assert_eq!(abilities.options_for(AbilitySlot::Ultimate), expected);
    }

    #[test]
    fn has_a_rune_profile_with_fire_affinity() {
        let item = SwiftBoots;
        let profile = item.rune_profile().expect("swift_boots must grant a rune profile");
        assert_eq!(profile.capacity, 5);
        assert_eq!(profile.affinity.as_ref().map(|id| id.as_str()), Some("fuoco"));
    }

    #[test]
    fn grants_armor_and_speed_bonuses() {
        let item = SwiftBoots;
        assert_eq!(item.effects().len(), 2);
    }
}
