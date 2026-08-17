//! Gauntlets weapon family.

pub mod battle_gauntlets;
pub mod iron_fists;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::strike::Strike;
use crate::ability_definitions::rush::Rush;
use crate::ability_definitions::impact::Impact;

#[weapon_family(
    id = "gauntlets",
    name = "Gauntlets",
    primary = [Strike],
    secondary = [Rush],
    ultimate = [Impact],
)]
pub struct GauntletsFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    GauntletsFamily::register(registry);
}
