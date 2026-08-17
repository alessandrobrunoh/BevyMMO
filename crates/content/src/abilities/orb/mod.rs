//! Orb — Focus primary ability (Q).
//!
//! Projects a concentrated orb of energy that homes toward a target.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "orb",
    name = "Orb",
    tags = [Ranged, Projectile, SingleTarget, RepeatCompatible, EchoCompatible],
    range = 20.0,
    geometry = projectile(speed = 22.0),
    potency = 150.0,
    cast_time = 0.25,
    cooldown = 2.5,
    energy_cost = 9.0,
    animation = "focus_orb",
    impact_vfx = "orb_impact",
)]
pub struct Orb;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Orb::register(registry);
}
