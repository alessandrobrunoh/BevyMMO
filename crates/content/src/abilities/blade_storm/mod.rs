//! Blade Storm — Sword ultimate ability (E).
//!
//! Becomes a whirlwind of steel, dealing rapid damage to all nearby enemies.
//! The spinning blades leave deep, burning cuts on anyone caught within.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "blade_storm",
    name = "Blade Storm",
    tags = [Melee, Area],
    range = 5.5,
    geometry = circle(radius = 5.5),
    potency = 235.0,
    cast_time = 0.7,
    cooldown = 22.0,
    energy_cost = 40.0,
    statuses = [Burn],
    animation = "sword_ultimate",
    impact_vfx = "blade_storm_impact",
)]
pub struct BladeStorm;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    BladeStorm::register(registry);
}
