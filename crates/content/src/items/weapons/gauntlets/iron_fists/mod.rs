//! Iron Fists — a Gauntlets item whose signature execution is Echo.

use bevymmo_props_macro::item;

use crate::ability_definitions::strike::Strike;
use crate::ability_definitions::rush::Rush;
use crate::ability_definitions::impact::Impact;
use crate::items::ItemRegistry;

#[item(
    id = "iron_fists",
    name = "Iron Fists",
    description = "Reinforced gauntlets that echo rapid strikes with deadly precision.",
    category = Weapon,
    rarity = Epic,
    slot = Weapon,
    family = Gauntlets,
    execution = Echo,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 58.0)],
    abilities(
        primary = [Strike],
        secondary = [Rush],
        ultimate = [Impact],
    ),
    rune_profile(capacity = 12, stability = 0.83),
)]
pub struct IronFists;

pub fn register(registry: &mut ItemRegistry) {
    IronFists::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::{AbilityTag, BlueprintExecution};
    use bevymmo_gameplay::items::Item;

    #[test]
    fn marks_the_blueprint_as_echo_execution() {
        let blueprint = IronFists.ability_blueprint(&Strike);
        assert_eq!(blueprint.execution, BlueprintExecution::Echo);
        assert!(blueprint.has_tag(AbilityTag::RepeatCompatible));
    }

    #[test]
    fn rush_is_melee_area_ability() {
        let blueprint = IronFists.ability_blueprint(&Rush);
        assert!(blueprint.has_tag(AbilityTag::Melee));
        assert!(blueprint.has_tag(AbilityTag::Area));
    }
}
