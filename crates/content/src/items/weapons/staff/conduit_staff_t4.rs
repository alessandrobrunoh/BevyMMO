//! Conduit Staff T4 — a Staff item whose signature execution is Charge.

use bevymmo_props_macro::item;

use crate::ability_definitions::arcane_orb::ArcaneOrb;
use crate::ability_definitions::astral_nova::AstralNova;
use crate::ability_definitions::meteor_lance::MeteorLance;
use crate::items::ItemRegistry;


#[item(
    id = "conduit_staff_t4",
    name = "Conduit Staff T4",
    description = "Un bastone che carica il gesto prima di manifestarlo.",
    category = Weapon,
    rarity = Epic,
    slot = Weapon,
    family = Staff,
    execution = Charge,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 80.0)],
    abilities(
        primary = [ArcaneOrb],
        secondary = [ArcaneOrb],
        ultimate = [AstralNova, MeteorLance],
    ),
    rune_profile(capacity = 12, stability = 0.85, affinity = fuoco),
)]
pub struct ConduitStaffT4;

pub fn register(registry: &mut ItemRegistry) {
    ConduitStaffT4::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::BlueprintExecution;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn transforms_the_base_blueprint_into_charge_execution() {
        let blueprint = ConduitStaffT4.ability_blueprint(&ArcaneOrb);
        assert_eq!(blueprint.execution, BlueprintExecution::Charge);
    }
}
