//! Robust Cuirass — a basic chestplate offering balanced protection.

use bevymmo_props_macro::item;

use crate::items::ItemRegistry;

#[item(
    id = "robust_cuirass",
    name = "Robust Cuirass",
    description = "A sturdy leather cuirass reinforced with iron rivets. Provides reliable protection without sacrificing mobility.",
    category = Armor,
    rarity = Uncommon,
    slot = Armor,
    tradable = true,
    effects = [stat_bonus(field = Armor, op = Add, value = 15.0)],
    rune_profile(capacity = 6, stability = 0.92),
)]
pub struct RobustCuirass;

pub fn register(registry: &mut ItemRegistry) {
    RobustCuirass::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::components::EquipSlot;
    use crate::items::definition::Item;

    #[test]
    fn id_and_slot_match_design() {
        let item = RobustCuirass;
        assert_eq!(item.id().as_str(), "robust_cuirass");
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Armor));
    }

    #[test]
    fn category_is_armor() {
        let item = RobustCuirass;
        assert!(matches!(
            item.config().category,
            crate::items::ItemCategory::Armor
        ));
    }

    #[test]
    fn has_no_ability_loadout() {
        assert!(RobustCuirass.ability_loadout().is_none());
    }

    #[test]
    fn has_a_rune_profile_with_stability() {
        let item = RobustCuirass;
        let profile = item
            .rune_profile()
            .expect("robust_cuirass must grant a rune profile");
        assert_eq!(profile.capacity, 6);
        assert!((profile.stability - 0.92).abs() < f32::EPSILON);
    }

    #[test]
    fn grants_armor_stat_bonus() {
        let item = RobustCuirass;
        assert_eq!(item.effects().len(), 1);
    }
}
