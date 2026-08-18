//! Orb — Focus primary ability (Q).
//!
//! Projects a concentrated orb of energy that homes toward a target.
//! The orb moves slowly but burns with sustained arcane fire on impact.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "orb",
    name = "Orb",
    tags = [Ranged, Projectile, SingleTarget, RepeatCompatible, EchoCompatible],
    range = 18.0,
    geometry = projectile(speed = 18.0),
    potency = 135.0,
    cast_time = 0.3,
    cooldown = 2.8,
    energy_cost = 10.0,
    statuses = [Burn],
    animation = "focus_orb",
    impact_vfx = "orb_impact",
)]
pub struct Orb;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Orb::register(registry);
}
