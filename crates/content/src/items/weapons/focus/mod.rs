//! Focus weapon family.

pub mod arcane_focus;
pub mod primal_focus;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::orb::Orb;
use crate::ability_definitions::field::Field;
use crate::ability_definitions::domain::Domain;

#[weapon_family(
    id = "focus",
    name = "Focus",
    primary = [Orb],
    secondary = [Field],
    ultimate = [Domain],
)]
pub struct FocusFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    FocusFamily::register(registry);
}
