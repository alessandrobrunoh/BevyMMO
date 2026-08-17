//! Rush — Gauntlets secondary ability (W).
//!
//! Dashes forward in a short burst, striking all enemies in your path.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "rush",
    name = "Rush",
    tags = [Melee, Area],
    range = 6.0,
    geometry = cone(radius = 5.0, angle_deg = 50.0),
    potency = 110.0,
    cast_time = 0.25,
    cooldown = 5.0,
    energy_cost = 14.0,
    animation = "gauntlet_rush",
    impact_vfx = "rush_impact",
)]
pub struct Rush;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Rush::register(registry);
}
