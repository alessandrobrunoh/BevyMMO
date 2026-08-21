//! Sword weapon.

use bevymmo_props_macro::item;

use crate::ability_definitions::blade_storm::BladeStorm;
use crate::ability_definitions::cleave::Cleave;
use crate::ability_definitions::lunge::Lunge;
use crate::items::ItemRegistry;

#[item(
    id = "sword",
    name = "Spada",
    description = "A balanced sword with a wide cleave, a thrusting lunge, and a whirling ultimate.",
    category = Weapon,
    rarity = Rare,
    slot = Weapon,
    family = Sword,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 70.0)],
    abilities(
        primary = [Cleave],
        secondary = [Lunge],
        ultimate = [BladeStorm],
    ),
    rune_profile(capacity = 11, stability = 0.86),
)]
pub struct Sword;

pub fn register(registry: &mut ItemRegistry) {
    Sword::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::items::Item;

    #[test]
    fn offers_the_sword_loadout() {
        let loadout = Sword.ability_loadout().expect("sword offers gestures");
        assert_eq!(loadout.primary.len(), 1);
        assert_eq!(loadout.secondary.len(), 1);
        assert_eq!(loadout.ultimate.len(), 1);
    }
}
