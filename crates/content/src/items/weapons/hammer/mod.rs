//! Hammer weapon family.

pub mod warhammer;
pub mod maul;

use bevymmo_props_macro::weapon_family;

use crate::ability_definitions::crushing_blow::CrushingBlow;
use crate::ability_definitions::ground_slam::GroundSlam;
use crate::ability_definitions::cataclysm::Cataclysm;

#[weapon_family(
    id = "hammer",
    name = "Hammer",
    primary = [CrushingBlow],
    secondary = [GroundSlam],
    ultimate = [Cataclysm],
)]
pub struct HammerFamily;

pub fn register(registry: &mut crate::items::WeaponFamilyRegistry) {
    HammerFamily::register(registry);
}
