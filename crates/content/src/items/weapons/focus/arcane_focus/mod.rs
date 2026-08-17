//! Arcane Focus — a Focus item whose signature execution is Charge.

use bevymmo_props_macro::item;

use crate::ability_definitions::orb::Orb;
use crate::ability_definitions::field::Field;
use crate::ability_definitions::domain::Domain;
use crate::items::ItemRegistry;

#[item(
    id = "arcane_focus",
    name = "Arcane Focus",
    description = "A crystalline focus that channels charged arcane energy.",
    category = Weapon,
    rarity = Epic,
    slot = Weapon,
    family = Focus,
    execution = Charge,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 60.0)],
    abilities(
        primary = [Orb],
        secondary = [Field],
        ultimate = [Domain],
    ),
    rune_profile(capacity = 13, stability = 0.80),
)]
pub struct ArcaneFocus;

pub fn register(registry: &mut ItemRegistry) {
    ArcaneFocus::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = ArcaneFocus.ability_blueprint(&Orb);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
