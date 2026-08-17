//! Cleave — Sword primary ability (Q).
//!
//! A sweeping melee strike that hits all enemies in a wide frontal arc.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "cleave",
    name = "Cleave",
    tags = [Melee, Area],
    range = 4.5,
    geometry = cone(radius = 4.5, angle_deg = 75.0),
    potency = 130.0,
    cast_time = 0.3,
    cooldown = 3.5,
    energy_cost = 10.0,
    animation = "sword_cleave",
    impact_vfx = "cleave_impact",
)]
pub struct Cleave;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Cleave::register(registry);
}
