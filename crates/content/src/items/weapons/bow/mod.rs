//! Bow weapon family.

pub mod bow;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::piercing_barrage::PiercingBarrage;
use crate::ability_definitions::power_shot::PowerShot;
use crate::ability_definitions::volley::Volley;

#[weapon_family(
    id = "bow",
    name = "Arco",
    primary = [PowerShot],
    secondary = [Volley],
    ultimate = [PiercingBarrage],
)]
pub struct BowFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    BowFamily::register(registry);
}
