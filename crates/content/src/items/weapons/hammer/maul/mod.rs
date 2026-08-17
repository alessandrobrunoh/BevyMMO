//! Maul — a Hammer item whose signature execution is Echo.

use bevymmo_props_macro::item;

use crate::ability_definitions::crushing_blow::CrushingBlow;
use crate::ability_definitions::ground_slam::GroundSlam;
use crate::ability_definitions::cataclysm::Cataclysm;
use crate::items::ItemRegistry;

#[item(
    id = "maul",
    name = "Maul",
    description = "A brutal maul that echoes each devastating impact.",
    category = Weapon,
    rarity = Epic,
    slot = Weapon,
    family = Hammer,
    execution = Echo,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 75.0)],
    abilities(
        primary = [CrushingBlow],
        secondary = [GroundSlam],
        ultimate = [Cataclysm],
    ),
    rune_profile(capacity = 11, stability = 0.84),
)]
pub struct Maul;

pub fn register(registry: &mut ItemRegistry) {
    Maul::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::{AbilityTag, BlueprintExecution};
    use bevymmo_gameplay::items::Item;

    #[test]
    fn marks_the_blueprint_as_echo_execution() {
        let blueprint = Maul.ability_blueprint(&CrushingBlow);
        assert_eq!(blueprint.execution, BlueprintExecution::Echo);
        assert!(blueprint.has_tag(AbilityTag::Melee));
    }

    #[test]
    fn ground_slam_has_ground_tag() {
        let blueprint = Maul.ability_blueprint(&GroundSlam);
        assert!(blueprint.has_tag(AbilityTag::Ground));
    }
}
