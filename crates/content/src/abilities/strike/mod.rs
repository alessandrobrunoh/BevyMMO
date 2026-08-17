//! Strike — Gauntlets primary ability (Q).
//!
//! A quick, precise punch that strikes a single target with focused force.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "strike",
    name = "Strike",
    tags = [Melee, SingleTarget, RepeatCompatible],
    range = 3.5,
    geometry = cone(radius = 3.0, angle_deg = 35.0),
    potency = 120.0,
    cast_time = 0.1,
    cooldown = 1.8,
    energy_cost = 6.0,
    animation = "gauntlet_strike",
    impact_vfx = "strike_impact",
)]
pub struct Strike;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Strike::register(registry);
}
