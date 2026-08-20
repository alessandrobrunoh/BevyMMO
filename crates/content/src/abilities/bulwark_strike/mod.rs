//! Bulwark Strike — chestplate primary ability.

use crate::abilities::BaseAbilityRegistry;
use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "bulwark_strike",
    name = "Bulwark Strike",
    tags = [Melee, SingleTarget],
    range = 4.5,
    geometry = cone(radius = 4.5, angle_deg = 80.0),
    potency = 155.0,
    cast_time = 0.25,
    cooldown = 6.0,
    energy_cost = 10.0,
    animation = "cuirass_bulwark_strike",
    impact_vfx = "bulwark_strike_impact",
)]
pub struct BulwarkStrike;

pub fn register(registry: &mut BaseAbilityRegistry) {
    BulwarkStrike::register(registry);
}
