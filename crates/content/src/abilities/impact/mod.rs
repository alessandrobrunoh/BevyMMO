//! Impact — Gauntlets ultimate ability (E).
//!
//! Gathers all your strength for a single devastating punch that sends
//! shockwaves through anything caught in the blast.
//! The delayed release creates a moment of tension before devastation.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "impact",
    name = "Impact",
    tags = [Melee, Area, Ground],
    range = 6.0,
    geometry = circle(radius = 7.0),
    potency = 320.0,
    cast_time = 0.9,
    cooldown = 24.0,
    energy_cost = 46.0,
    statuses = [Stun],
    stun_seconds = 1.8,
    impact_delay = 0.35,
    animation = "gauntlet_ultimate",
    impact_vfx = "impact_impact",
)]
pub struct Impact;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Impact::register(registry);
}
