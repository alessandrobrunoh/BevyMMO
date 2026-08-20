//! Sword weapon family.

pub mod sword;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::blade_storm::BladeStorm;
use crate::ability_definitions::cleave::Cleave;
use crate::ability_definitions::lunge::Lunge;

#[weapon_family(
    id = "sword",
    name = "Spada",
    primary = [Cleave],
    secondary = [Lunge],
    ultimate = [BladeStorm],
)]
pub struct SwordFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    SwordFamily::register(registry);
}
