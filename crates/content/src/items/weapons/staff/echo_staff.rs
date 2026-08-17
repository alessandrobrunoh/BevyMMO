//! Echo Staff — repeats eligible manifestations once.

use bevymmo_props_macro::item;

use crate::ability_definitions::arcane_orb::ArcaneOrb;
use crate::ability_definitions::astral_nova::AstralNova;
use crate::ability_definitions::meteor_lance::MeteorLance;
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
        primary = [ArcaneOrb],
        secondary = [ArcaneOrb],
        ultimate = [MeteorLance, AstralNova],
    ),
    rune_profile(capacity = 14, stability = 0.78, affinity = fuoco),
)]
pub struct EchoStaff;

pub fn register(registry: &mut ItemRegistry) {
    EchoStaff::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::{AbilitySlot, BlueprintExecution};
    use bevymmo_gameplay::items::Item;

    #[test]
    fn marks_the_blueprint_as_echo_execution() {
        let blueprint = EchoStaff.ability_blueprint(&ArcaneOrb);
        assert_eq!(blueprint.execution, BlueprintExecution::Echo);
        assert!(blueprint.has_tag(bevymmo_gameplay::abilities::AbilityTag::EchoCompatible));
    }

    #[test]
    fn offers_two_ultimate_choices() {
        let abilities = EchoStaff
            .ability_loadout()
            .expect("Echo Staff has abilities");
        assert_eq!(
            abilities.options_for(AbilitySlot::Ultimate),
            [MeteorLance::ID.into(), AstralNova::ID.into()]
        );
    }
}
