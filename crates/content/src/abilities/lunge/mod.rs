//! Lunge — Sword secondary ability (W).
//!
//! A quick forward thrust that strikes a single target with precision.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "lunge",
    name = "Lunge",
    tags = [Melee, SingleTarget, RepeatCompatible],
    range = 5.0,
    geometry = cone(radius = 3.0, angle_deg = 30.0),
    potency = 160.0,
    cast_time = 0.15,
    cooldown = 2.5,
    energy_cost = 8.0,
    animation = "sword_lunge",
    impact_vfx = "lunge_impact",
)]
pub struct Lunge;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Lunge::register(registry);
}
