//! "Purity Charm" — offhand accessory.

use bevymmo_props_macro::item;

use crate::items::ItemRegistry;

#[item(
    id = "purity_charm",
    name = "Purity Charm",
    description = "A blessed charm that radiates faint light.",
    category = Accessory,
    rarity = Rare,
    slot = Offhand,
    tradable = true,
    rune_profile(capacity = 4, stability = 0.98),
)]
pub struct PurityCharm;

/// Adds this content package to the item registry.
pub fn register(registry: &mut ItemRegistry) {
    PurityCharm::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn has_no_ability_loadout() {
        assert!(PurityCharm.ability_loadout().is_none());
    }

    #[test]
    fn has_a_rune_profile_with_stability() {
        let item = PurityCharm;
        let profile = item
            .rune_profile()
            .expect("purity_charm must grant a rune profile");
        assert_eq!(profile.capacity, 4);
        assert!((profile.stability - 0.98).abs() < f32::EPSILON);
    }
}
