//! Blade Storm — Sword ultimate ability (E).
//!
//! Becomes a whirlwind of steel, dealing rapid damage to all nearby enemies.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "blade_storm",
    name = "Blade Storm",
    tags = [Melee, Area],
    range = 5.0,
    geometry = circle(radius = 5.0),
    potency = 260.0,
    cast_time = 0.6,
    cooldown = 20.0,
    energy_cost = 38.0,
    animation = "sword_ultimate",
    impact_vfx = "blade_storm_impact",
)]
pub struct BladeStorm;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    BladeStorm::register(registry);
}
