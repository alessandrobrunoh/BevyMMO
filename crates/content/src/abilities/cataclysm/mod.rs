//! Cataclysm — Hammer ultimate ability (E).
//!
//! Brings the hammer down with earth-shattering force, creating a massive
//! devastation zone that cripples anything caught within.
//! The seismic event stuns everyone in its radius.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "cataclysm",
    name = "Cataclysm",
    tags = [Melee, Area, Ground],
    range = 7.0,
    geometry = circle(radius = 8.0),
    potency = 380.0,
    cast_time = 1.3,
    cooldown = 30.0,
    energy_cost = 55.0,
    stun_seconds = 2.0,
    impact_delay = 0.5,
    animation = "hammer_ultimate",
    impact_vfx = "cataclysm_impact",
)]
pub struct Cataclysm;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Cataclysm::register(registry);
}
