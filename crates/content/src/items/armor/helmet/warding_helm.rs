//! Warding Helm — a helmet providing head protection with arcane resonance.

use bevymmo_props_macro::item;

use crate::ability_definitions::arcane_orb::ArcaneOrb;
use crate::items::ItemRegistry;

#[item(
    id = "warding_helm",
    name = "Warding Helm",
    description = "A helmet inscribed with basic warding glyphs. Protects the wearer's mind as well as their skull.",
    category = Armor,
    rarity = Rare,
    slot = Helmet,
    effects = [
        stat_bonus(field = Armor, op = Add, value = 10.0),
        stat_bonus(field = MaxHealth, op = Add, value = 50.0),
    ],
    abilities(
        primary = [ArcaneOrb],
        secondary = [ArcaneOrb],
        ultimate = [ArcaneOrb],
    ),
    rune_profile(capacity = 7, stability = 0.90, affinity = fuoco),
)]
pub struct WardingHelm;

pub fn register(registry: &mut ItemRegistry) {
    WardingHelm::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilitySlot;
    use crate::items::components::EquipSlot;
    use crate::items::definition::Item;

    #[test]
    fn id_and_slot_match_design() {
        let item = WardingHelm;
        assert_eq!(item.id().as_str(), "warding_helm");
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Helmet));
    }

    #[test]
    fn category_is_armor() {
        let item = WardingHelm;
        assert!(matches!(item.config().category, crate::items::ItemCategory::Armor));
    }

    #[test]
    fn offers_arcane_orb_for_every_slot() {
        let item = WardingHelm;
        let abilities = item
            .ability_loadout()
            .expect("warding_helm must grant abilities");
        let expected = [ArcaneOrb::ID.into()];

        assert_eq!(abilities.options_for(AbilitySlot::Primary), expected);
        assert_eq!(abilities.options_for(AbilitySlot::Secondary), expected);
        assert_eq!(abilities.options_for(AbilitySlot::Ultimate), expected);
    }

    #[test]
    fn has_a_rune_profile_with_fire_affinity() {
        let item = WardingHelm;
        let profile = item.rune_profile().expect("warding_helm must grant a rune profile");
        assert_eq!(profile.capacity, 7);
        assert_eq!(profile.affinity.as_ref().map(|id| id.as_str()), Some("fuoco"));
    }

    #[test]
    fn grants_armor_and_health_bonuses() {
        let item = WardingHelm;
        assert_eq!(item.effects().len(), 2);
    }
}
