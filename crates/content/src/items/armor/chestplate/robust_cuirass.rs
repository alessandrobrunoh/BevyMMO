//! Robust Cuirass — a basic chestplate offering balanced protection.

use bevymmo_props_macro::item;

use crate::ability_definitions::arcane_orb::ArcaneOrb;
use crate::ability_definitions::bulwark_strike::BulwarkStrike;
use crate::ability_definitions::iron_wave::IronWave;
use crate::items::ItemRegistry;

#[item(
    id = "robust_cuirass",
    name = "Robust Cuirass",
    description = "A sturdy leather cuirass reinforced with iron rivets. Provides reliable protection without sacrificing mobility.",
    category = Armor,
    rarity = Uncommon,
    slot = Armor,
    effects = [stat_bonus(field = Armor, op = Add, value = 15.0)],
    abilities(
        primary = [BulwarkStrike],
        secondary = [IronWave],
        ultimate = [ArcaneOrb],
    ),
    rune_profile(capacity = 6, stability = 0.92, affinity = fuoco),
)]
pub struct RobustCuirass;

pub fn register(registry: &mut ItemRegistry) {
    RobustCuirass::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilitySlot;
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
        assert!(matches!(item.config().category, crate::items::ItemCategory::Armor));
    }

    #[test]
    fn offers_dedicated_chestplate_abilities() {
        let item = RobustCuirass;
        let abilities = item
            .ability_loadout()
            .expect("robust_cuirass must grant abilities");
        assert_eq!(abilities.options_for(AbilitySlot::Primary), [BulwarkStrike::ID.into()]);
        assert_eq!(abilities.options_for(AbilitySlot::Secondary), [IronWave::ID.into()]);
        assert_eq!(abilities.options_for(AbilitySlot::Ultimate), [ArcaneOrb::ID.into()]);
    }

    #[test]
    fn has_a_rune_profile_with_fire_affinity() {
        let item = RobustCuirass;
        let profile = item.rune_profile().expect("robust_cuirass must grant a rune profile");
        assert_eq!(profile.capacity, 6);
        assert_eq!(profile.affinity.as_ref().map(|id| id.as_str()), Some("fuoco"));
    }

    #[test]
    fn grants_armor_stat_bonus() {
        let item = RobustCuirass;
        assert_eq!(item.effects().len(), 1);
    }
}
