//! Bow weapon.

use bevymmo_props_macro::item;

use crate::ability_definitions::piercing_barrage::PiercingBarrage;
use crate::ability_definitions::power_shot::PowerShot;
use crate::ability_definitions::volley::Volley;
use crate::items::ItemRegistry;

#[item(
    id = "bow",
    name = "Arco",
    description = "A powerful bow that charges shots for devastating impact.",
    category = Weapon,
    rarity = Rare,
    slot = Weapon,
    family = Bow,
    execution = Charge,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 65.0)],
    abilities(
        primary = [PowerShot],
        secondary = [Volley],
        ultimate = [PiercingBarrage],
    ),
    rune_profile(capacity = 10, stability = 0.88),
)]
pub struct Bow;

pub fn register(registry: &mut ItemRegistry) {
    Bow::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = Bow.ability_blueprint(&PowerShot);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
