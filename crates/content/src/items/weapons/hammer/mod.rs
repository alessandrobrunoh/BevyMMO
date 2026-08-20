//! Hammer weapon family.

pub mod hammer;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::cataclysm::Cataclysm;
use crate::ability_definitions::crushing_blow::CrushingBlow;
use crate::ability_definitions::ground_slam::GroundSlam;

#[weapon_family(
    id = "hammer",
    name = "Martello",
    primary = [CrushingBlow],
    secondary = [GroundSlam],
    ultimate = [Cataclysm],
)]
pub struct HammerFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    HammerFamily::register(registry);
}
