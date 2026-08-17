//! Arming Sword — a Sword item whose signature execution is Charge.

use bevymmo_props_macro::item;

use crate::ability_definitions::cleave::Cleave;
use crate::ability_definitions::lunge::Lunge;
use crate::ability_definitions::blade_storm::BladeStorm;
use crate::items::ItemRegistry;

#[item(
    id = "arming_sword",
    name = "Arming Sword",
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
pub struct ArmingSword;

pub fn register(registry: &mut ItemRegistry) {
    ArmingSword::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = ArmingSword.ability_blueprint(&Cleave);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
