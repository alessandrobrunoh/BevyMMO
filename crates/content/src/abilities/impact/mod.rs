//! Impact — Gauntlets ultimate ability (E).
//!
//! Gathers all your strength for a single devastating punch that sends
//! shockwaves through anything caught in the blast.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "impact",
    name = "Impact",
    tags = [Melee, Area, Ground],
    range = 5.0,
    geometry = circle(radius = 6.0),
    potency = 290.0,
    cast_time = 0.7,
    cooldown = 22.0,
    energy_cost = 44.0,
    animation = "gauntlet_ultimate",
    impact_vfx = "impact_impact",
)]
pub struct Impact;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Impact::register(registry);
}
