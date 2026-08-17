//! Crushing Blow — Hammer primary ability (Q).
//!
//! A devastating overhead strike that crushes a single target with immense force.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "crushing_blow",
    name = "Crushing Blow",
    tags = [Melee, SingleTarget],
    range = 4.0,
    geometry = cone(radius = 3.5, angle_deg = 40.0),
    potency = 190.0,
    cast_time = 0.4,
    cooldown = 4.0,
    energy_cost = 12.0,
    animation = "hammer_smash",
    impact_vfx = "crushing_blow_impact",
)]
pub struct CrushingBlow;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    CrushingBlow::register(registry);
}
