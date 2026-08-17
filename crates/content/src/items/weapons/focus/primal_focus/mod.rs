//! Primal Focus — a Focus item whose signature execution is Echo.

use bevymmo_props_macro::item;

use crate::ability_definitions::orb::Orb;
use crate::ability_definitions::field::Field;
use crate::ability_definitions::domain::Domain;
use crate::items::ItemRegistry;

#[item(
    id = "primal_focus",
    name = "Primal Focus",
    description = "A primal focus that echoes manifestations of raw energy.",
    category = Weapon,
    rarity = Legendary,
    slot = Weapon,
    family = Focus,
    execution = Echo,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 55.0)],
    abilities(
        primary = [Orb],
        secondary = [Field],
        ultimate = [Domain],
    ),
    rune_profile(capacity = 15, stability = 0.74),
)]
pub struct PrimalFocus;

pub fn register(registry: &mut ItemRegistry) {
    PrimalFocus::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::{AbilitySlot, BlueprintExecution};
    use bevymmo_gameplay::items::Item;

    #[test]
    fn marks_the_blueprint_as_echo_execution() {
        let blueprint = PrimalFocus.ability_blueprint(&Orb);
        assert_eq!(blueprint.execution, BlueprintExecution::Echo);
        assert!(blueprint.has_tag(bevymmo_gameplay::abilities::AbilityTag::EchoCompatible));
    }

    #[test]
    fn offers_domain_as_ultimate() {
        let abilities = PrimalFocus
            .ability_loadout()
            .expect("Primal Focus has abilities");
        assert_eq!(
            abilities.options_for(AbilitySlot::Ultimate),
            [Domain::ID.into()]
        );
    }
}
