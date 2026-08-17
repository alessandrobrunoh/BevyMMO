//! Cataclysm — Hammer ultimate ability (E).
//!
//! Brings the hammer down with earth-shattering force, creating a massive
//! devastation zone that cripples anything caught within.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "cataclysm",
    name = "Cataclysm",
    tags = [Melee, Area, Ground],
    range = 6.0,
    geometry = circle(radius = 7.0),
    potency = 340.0,
    cast_time = 1.0,
    cooldown = 28.0,
    energy_cost = 50.0,
    statuses = [Root],
    animation = "hammer_ultimate",
    impact_vfx = "cataclysm_impact",
)]
pub struct Cataclysm;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Cataclysm::register(registry);
}
