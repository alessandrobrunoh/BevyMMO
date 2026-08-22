//! A matching common armor set: helm, cape, cuirass, buckler, boots.
//!
//! Helm, chest and boots carry a small rune profile so they can take a Root
//! Word. Cape and buckler stay stat-only.

use bevymmo_props_macro::item;

use crate::items::ItemRegistry;

#[item(
    id = "simple_helm",
    name = "Simple Helm",
    description = "A plain iron cap. Better than nothing.",
    category = Armor,
    rarity = Common,
    slot = Helmet,
    tradable = true,
    effects = [stat_bonus(field = Armor, op = Add, value = 8.0)],
    rune_profile(capacity = 6, stability = 0.90),
)]
pub struct SimpleHelm;

#[item(
    id = "simple_cape",
    name = "Simple Cape",
    description = "A heavy cloth cloak that turns a glancing blow.",
    category = Armor,
    rarity = Common,
    slot = Cape,
    tradable = true,
    effects = [stat_bonus(field = Armor, op = Add, value = 4.0)],
)]
pub struct SimpleCape;

#[item(
    id = "simple_cuirass",
    name = "Simple Cuirass",
    description = "Riveted leather over a thin iron plate.",
    category = Armor,
    rarity = Common,
    slot = Armor,
    tradable = true,
    effects = [stat_bonus(field = Armor, op = Add, value = 12.0)],
    rune_profile(capacity = 6, stability = 0.92),
)]
pub struct SimpleCuirass;

#[item(
    id = "simple_buckler",
    name = "Simple Buckler",
    description = "A small round shield for the off hand.",
    category = Armor,
    rarity = Common,
    slot = Offhand,
    tradable = true,
    effects = [stat_bonus(field = Armor, op = Add, value = 8.0)],
)]
pub struct SimpleBuckler;

#[item(
    id = "simple_boots",
    name = "Simple Boots",
    description = "Stiff leather boots with iron toe caps.",
    category = Armor,
    rarity = Common,
    slot = Shoes,
    tradable = true,
    effects = [stat_bonus(field = Armor, op = Add, value = 4.0)],
    rune_profile(capacity = 5, stability = 0.94),
)]
pub struct SimpleBoots;

pub fn register(registry: &mut ItemRegistry) {
    SimpleHelm::register(registry);
    SimpleCape::register(registry);
    SimpleCuirass::register(registry);
    SimpleBuckler::register(registry);
    SimpleBoots::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::components::EquipSlot;
    use crate::items::definition::Item;

    #[test]
    fn every_worn_slot_has_a_simple_piece() {
        assert_eq!(SimpleHelm.config().equippable_into, Some(EquipSlot::Helmet));
        assert_eq!(SimpleCape.config().equippable_into, Some(EquipSlot::Cape));
        assert_eq!(
            SimpleCuirass.config().equippable_into,
            Some(EquipSlot::Armor)
        );
        assert_eq!(
            SimpleBuckler.config().equippable_into,
            Some(EquipSlot::Offhand)
        );
        assert_eq!(SimpleBoots.config().equippable_into, Some(EquipSlot::Shoes));
    }

    #[test]
    fn helm_chest_and_boots_are_inscribable_but_have_no_gestures() {
        assert!(SimpleHelm.ability_loadout().is_none());
        assert!(SimpleHelm.rune_profile().is_some());
        assert!(SimpleCuirass.ability_loadout().is_none());
        assert!(SimpleCuirass.rune_profile().is_some());
        assert!(SimpleBoots.ability_loadout().is_none());
        assert!(SimpleBoots.rune_profile().is_some());
    }

    #[test]
    fn cape_and_buckler_stay_stat_only() {
        assert!(SimpleCape.ability_loadout().is_none());
        assert!(SimpleBuckler.ability_loadout().is_none());
    }

    #[test]
    fn simple_pieces_are_tradable() {
        assert!(SimpleHelm.tradable());
        assert!(SimpleCape.tradable());
        assert!(SimpleCuirass.tradable());
        assert!(SimpleBuckler.tradable());
        assert!(SimpleBoots.tradable());
    }
}
