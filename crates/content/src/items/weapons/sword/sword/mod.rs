//! Sword weapon.

use bevymmo_props_macro::item;

use crate::ability_definitions::blade_storm::BladeStorm;
use crate::ability_definitions::cleave::Cleave;
use crate::ability_definitions::lunge::Lunge;
use crate::items::ItemRegistry;

#[item(
    id = "sword",
    name = "Spada",
    description = "A balanced sword that charges strikes for maximum effect.",
    category = Weapon,
    rarity = Rare,
    slot = Weapon,
    family = Sword,
    execution = Charge,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 70.0)],
    abilities(
        primary = [Cleave],
        secondary = [Lunge],
        ultimate = [BladeStorm],
    ),
    rune_profile(capacity = 11, stability = 0.86),
)]
pub struct Sword;

pub fn register(registry: &mut ItemRegistry) {
    Sword::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = Sword.ability_blueprint(&Cleave);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
