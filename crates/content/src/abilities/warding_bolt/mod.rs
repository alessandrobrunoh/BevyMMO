//! Warding Bolt — helmet primary ability.

use crate::abilities::BaseAbilityRegistry;
use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "warding_bolt",
    name = "Warding Bolt",
    tags = [Ranged, Projectile, SingleTarget],
    range = 18.0,
    geometry = projectile(speed = 26.0, range = 18.0),
    potency = 95.0,
    cast_time = 0.15,
    cooldown = 4.0,
    energy_cost = 8.0,
    animation = "helmet_warding_bolt",
    impact_vfx = "warding_bolt_impact",
)]
pub struct WardingBolt;

pub fn register(registry: &mut BaseAbilityRegistry) {
    WardingBolt::register(registry);
}
