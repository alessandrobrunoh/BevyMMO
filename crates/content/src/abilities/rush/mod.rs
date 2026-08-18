//! Rush — Gauntlets secondary ability (W).
//!
//! Dashes forward in a short burst, striking all enemies in your path.
//! The collision briefly roots anyone caught in the charge.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "rush",
    name = "Rush",
    tags = [Melee, Area],
    range = 7.0,
    geometry = cone(radius = 6.0, angle_deg = 45.0),
    potency = 125.0,
    cast_time = 0.2,
    cooldown = 5.5,
    energy_cost = 15.0,
    statuses = [Root],
    stun_seconds = 0.5,
    animation = "gauntlet_rush",
    impact_vfx = "rush_impact",
)]
pub struct Rush;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Rush::register(registry);
}
