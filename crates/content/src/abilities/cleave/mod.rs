//! Cleave — Sword primary ability (Q).
//!
//! A sweeping melee strike that hits all enemies in a wide frontal arc.
//! The fluid motion leaves the wielder momentarily hastened.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "cleave",
    name = "Cleave",
    tags = [Melee, Area],
    range = 5.0,
    geometry = cone(radius = 5.0, angle_deg = 85.0),
    potency = 115.0,
    cast_time = 0.25,
    cooldown = 3.0,
    mana_cost = 9.0,
    animation = "sword_cleave",
    impact_vfx = "cleave_impact",
)]
pub struct Cleave;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Cleave::register(registry);
}
