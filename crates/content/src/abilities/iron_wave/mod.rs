//! Iron Wave — chestplate secondary ability.

use crate::abilities::BaseAbilityRegistry;
use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "iron_wave",
    name = "Iron Wave",
    tags = [Melee, Area],
    range = 5.0,
    geometry = circle(radius = 4.0, range = 5.0),
    potency = 110.0,
    cast_time = 0.5,
    cooldown = 10.0,
    energy_cost = 18.0,
    animation = "cuirass_iron_wave",
    impact_vfx = "iron_wave_impact",
)]
pub struct IronWave;

pub fn register(registry: &mut BaseAbilityRegistry) {
    IronWave::register(registry);
}
