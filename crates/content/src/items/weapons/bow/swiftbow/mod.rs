//! Swiftbow — a Bow item whose signature execution is Echo.

use bevymmo_props_macro::item;

use crate::ability_definitions::power_shot::PowerShot;
use crate::ability_definitions::volley::Volley;
use crate::ability_definitions::piercing_barrage::PiercingBarrage;
use crate::items::ItemRegistry;

#[item(
    id = "swiftbow",
    name = "Swiftbow",
    description = "A lightweight bow that can echo rapid shots.",
    category = Weapon,
    rarity = Epic,
    slot = Weapon,
    family = Bow,
    execution = Echo,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 50.0)],
    abilities(
        primary = [PowerShot],
        secondary = [Volley],
        ultimate = [PiercingBarrage],
    ),
    rune_profile(capacity = 12, stability = 0.82),
)]
pub struct Swiftbow;

pub fn register(registry: &mut ItemRegistry) {
    Swiftbow::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::{AbilityTag, BlueprintExecution};
    use bevymmo_gameplay::items::Item;

    #[test]
    fn marks_the_blueprint_as_echo_execution() {
        let blueprint = Swiftbow.ability_blueprint(&PowerShot);
        assert_eq!(blueprint.execution, BlueprintExecution::Echo);
        assert!(blueprint.has_tag(AbilityTag::RepeatCompatible));
    }

    #[test]
    fn offers_three_ability_slots() {
        let abilities = Swiftbow
            .ability_loadout()
            .expect("Swiftbow has abilities");
        assert!(abilities.options_for(bevymmo_gameplay::abilities::AbilitySlot::Primary).len() > 0);
        assert!(abilities.options_for(bevymmo_gameplay::abilities::AbilitySlot::Secondary).len() > 0);
        assert!(abilities.options_for(bevymmo_gameplay::abilities::AbilitySlot::Ultimate).len() > 0);
    }
}
