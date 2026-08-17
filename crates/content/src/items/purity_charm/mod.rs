//! "Purity Charm" — accessory that grants cleanse and purge abilities.

use bevymmo_props_macro::item;

use crate::ability_definitions::cleanse::CleanseAbility;
use crate::ability_definitions::purge::PurgeAbility;
use crate::items::ItemRegistry;

#[item(
    id = "purity_charm",
    name = "Purity Charm",
    description = "A blessed charm that radiates faint light. Allows the wearer to purify harmful effects from themselves or dispel beneficial magic from enemies.",
    category = Accessory,
    rarity = Rare,
    slot = Offhand,
    abilities(
        primary = [CleanseAbility],
        secondary = [PurgeAbility],
        ultimate = [CleanseAbility],
    ),
    rune_profile(capacity = 4, stability = 0.98, affinity = sacro),
)]
pub struct PurityCharm;

/// Adds this content package to the item registry.
pub fn register(registry: &mut ItemRegistry) {
    PurityCharm::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilitySlot;
    use crate::items::components::EquipSlot;
    use crate::items::definition::Item;

    #[test]
    fn id_and_slot_match_design() {
        let item = PurityCharm;
        assert_eq!(item.id().as_str(), "purity_charm");
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Offhand));
    }

    #[test]
    fn category_is_accessory() {
        let item = PurityCharm;
        assert!(matches!(
            item.config().category,
            crate::items::ItemCategory::Accessory
        ));
    }

    #[test]
    fn offers_cleanse_and_purge_abilities() {
        let item = PurityCharm;
        let abilities = item
            .ability_loadout()
            .expect("purity_charm must grant abilities");

        // Primary and Ultimate get Cleanse
        assert_eq!(
            abilities.options_for(AbilitySlot::Primary),
            [CleanseAbility::ID.into()]
        );
        assert_eq!(
            abilities.options_for(AbilitySlot::Ultimate),
            [CleanseAbility::ID.into()]
        );

        // Secondary gets Purge
        assert_eq!(
            abilities.options_for(AbilitySlot::Secondary),
            [PurgeAbility::ID.into()]
        );
    }

    #[test]
    fn has_a_rune_profile_with_holy_affinity() {
        let item = PurityCharm;
        let profile = item
            .rune_profile()
            .expect("purity_charm must grant a rune profile");
        assert_eq!(profile.capacity, 4);
        assert_eq!(
            profile.affinity.as_ref().map(|id| id.as_str()),
            Some("sacro")
        );
    }
}
