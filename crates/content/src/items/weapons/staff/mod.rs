//! Staff weapon family.

pub mod mage_staff;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::arcane_bolt::ArcaneBolt;
use crate::ability_definitions::arcane_wave::ArcaneWave;
use crate::ability_definitions::great_manifestation::GreatManifestation;

#[weapon_family(
    id = "staff",
    name = "Staffa da Mago",
    primary = [ArcaneBolt],
    secondary = [ArcaneWave],
    ultimate = [GreatManifestation],
)]
pub struct StaffFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    StaffFamily::register(registry);
}
