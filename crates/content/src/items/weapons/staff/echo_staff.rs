//! Echo Staff — repeats eligible manifestations once.

use bevymmo_props_macro::item;

use crate::ability_definitions::arcane_bolt::ArcaneBolt;
use crate::ability_definitions::arcane_wave::ArcaneWave;
use crate::ability_definitions::great_manifestation::GreatManifestation;
use crate::items::ItemRegistry;

#[item(
    id = "echo_staff",
    name = "Echo Staff",
    description = "Un bastone che ripete una manifestazione compatibile con l'Eco.",
    category = Weapon,
    rarity = Legendary,
    slot = Weapon,
    family = Staff,
    execution = Echo,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 55.0)],
    abilities(
        primary = [ArcaneBolt],
        secondary = [ArcaneWave],
        ultimate = [GreatManifestation],
    ),
    rune_profile(capacity = 14, stability = 0.78),
)]
pub struct EchoStaff;

pub fn register(registry: &mut ItemRegistry) {
    EchoStaff::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn marks_the_blueprint_as_echo_execution() {
        let blueprint = EchoStaff.ability_blueprint(&ArcaneBolt);
        assert_eq!(blueprint.execution, BlueprintExecution::Echo);
        assert!(blueprint.has_tag(bevymmo_gameplay::abilities::AbilityTag::EchoCompatible));
    }


}
