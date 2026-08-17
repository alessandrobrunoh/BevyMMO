//! Sword weapon family.

pub mod arming_sword;
pub mod greatsword;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::cleave::Cleave;
use crate::ability_definitions::lunge::Lunge;
use crate::ability_definitions::blade_storm::BladeStorm;

#[weapon_family(
    id = "sword",
    name = "Sword",
    primary = [Cleave],
    secondary = [Lunge],
    ultimate = [BladeStorm],
)]
pub struct SwordFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    SwordFamily::register(registry);
}
