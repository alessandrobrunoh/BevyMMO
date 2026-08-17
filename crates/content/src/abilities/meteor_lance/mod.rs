//! Meteor Lance — long-range projectile ultimate for staff weapons.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "meteor_lance",
    name = "Lancia Meteora",
    tags = [Ranged, Projectile, SingleTarget],
    range = 32.0,
    geometry = projectile(speed = 32.0, range = 32.0),
    potency = 620.0,
    cast_time = 0.85,
    cooldown = 20.0,
    energy_cost = 35.0,
    animation = "staff_meteor_cast",
    impact_vfx = "meteor_lance_impact",
)]
pub struct MeteorLance;

pub fn register(registry: &mut BaseAbilityRegistry) {
    MeteorLance::register(registry);
}
