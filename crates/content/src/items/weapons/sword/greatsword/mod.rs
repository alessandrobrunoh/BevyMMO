//! Greatsword — a Sword item whose signature execution is Echo.

use bevymmo_props_macro::item;

use crate::ability_definitions::cleave::Cleave;
use crate::ability_definitions::lunge::Lunge;
use crate::ability_definitions::blade_storm::BladeStorm;
use crate::items::ItemRegistry;

#[item(
    id = "greatsword",
    name = "Greatsword",
    description = "A massive two-handed sword that echoes its devastating strikes.",
    category = Weapon,
    rarity = Legendary,
    slot = Weapon,
    family = Sword,
    execution = Echo,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 85.0)],
    abilities(
        primary = [Cleave],
        secondary = [Lunge],
        ultimate = [BladeStorm],
    ),
    rune_profile(capacity = 14, stability = 0.76),
)]
pub struct Greatsword;

pub fn register(registry: &mut ItemRegistry) {
    Greatsword::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::{AbilityTag, BlueprintExecution};
    use bevymmo_gameplay::items::Item;

    #[test]
    fn marks_the_blueprint_as_echo_execution() {
        let blueprint = Greatsword.ability_blueprint(&Cleave);
        assert_eq!(blueprint.execution, BlueprintExecution::Echo);
        assert!(blueprint.has_tag(AbilityTag::Area));
    }
}
