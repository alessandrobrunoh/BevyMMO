//! Mage staff weapon.

use bevymmo_props_macro::item;

use crate::ability_definitions::arcane_bolt::ArcaneBolt;
use crate::ability_definitions::arcane_wave::ArcaneWave;
use crate::ability_definitions::great_manifestation::GreatManifestation;
use crate::items::ItemRegistry;

#[item(
    id = "mage_staff",
    name = "Staffa da Mago",
    description = "Un bastone che carica il gesto prima di manifestarlo.",
    category = Weapon,
    rarity = Epic,
    slot = Weapon,
    family = Staff,
    execution = Charge,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 80.0)],
    abilities(
        primary = [ArcaneBolt],
        secondary = [ArcaneWave],
        ultimate = [GreatManifestation],
    ),
    rune_profile(capacity = 12, stability = 0.85),
)]
pub struct MageStaff;

pub fn register(registry: &mut ItemRegistry) {
    MageStaff::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = MageStaff.ability_blueprint(&ArcaneBolt);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
