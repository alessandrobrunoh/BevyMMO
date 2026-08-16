//! Staff weapon family.

pub mod conduit_staff_t4;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::arcane_orb::ArcaneOrb;

#[weapon_family(
    id = "staff",
    name = "Staff",
    primary = [ArcaneOrb],
    secondary = [ArcaneOrb],
    ultimate = [ArcaneOrb],
)]
pub struct StaffFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    StaffFamily::register(registry);
}
