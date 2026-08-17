//! Battle Gauntlets — a Gauntlets item whose signature execution is Charge.

use bevymmo_props_macro::item;

use crate::ability_definitions::strike::Strike;
use crate::ability_definitions::rush::Rush;
use crate::ability_definitions::impact::Impact;
use crate::items::ItemRegistry;

#[item(
    id = "battle_gauntlets",
    name = "Battle Gauntlets",
    description = "Heavy gauntlets that charge each strike with immense force.",
    category = Weapon,
    rarity = Rare,
    slot = Weapon,
    family = Gauntlets,
    execution = Charge,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 72.0)],
    abilities(
        primary = [Strike],
        secondary = [Rush],
        ultimate = [Impact],
    ),
    rune_profile(capacity = 10, stability = 0.87),
)]
pub struct BattleGauntlets;

pub fn register(registry: &mut ItemRegistry) {
    BattleGauntlets::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = BattleGauntlets.ability_blueprint(&Strike);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
