//! Hammer weapon.

use bevymmo_props_macro::item;

use crate::ability_definitions::cataclysm::Cataclysm;
use crate::ability_definitions::crushing_blow::CrushingBlow;
use crate::ability_definitions::ground_slam::GroundSlam;
use crate::items::ItemRegistry;

#[item(
    id = "hammer",
    name = "Martello",
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
pub struct Hammer;

pub fn register(registry: &mut ItemRegistry) {
    Hammer::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = Hammer.ability_blueprint(&CrushingBlow);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
