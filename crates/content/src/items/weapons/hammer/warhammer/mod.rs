//! Warhammer — a Hammer item whose signature execution is Charge.

use bevymmo_props_macro::item;

use crate::ability_definitions::crushing_blow::CrushingBlow;
use crate::ability_definitions::ground_slam::GroundSlam;
use crate::ability_definitions::cataclysm::Cataclysm;
use crate::items::ItemRegistry;

#[item(
    id = "warhammer",
    name = "Warhammer",
    description = "A heavy hammer that charges before crushing blows.",
    category = Weapon,
    rarity = Rare,
    slot = Weapon,
    family = Hammer,
    execution = Charge,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 90.0)],
    abilities(
        primary = [CrushingBlow],
        secondary = [GroundSlam],
        ultimate = [Cataclysm],
    ),
    rune_profile(capacity = 9, stability = 0.91),
)]
pub struct Warhammer;

pub fn register(registry: &mut ItemRegistry) {
    Warhammer::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = Warhammer.ability_blueprint(&CrushingBlow);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
