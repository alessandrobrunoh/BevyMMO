//! Crushing Blow — Hammer primary ability (Q).
//!
//! A devastating overhead strike that crushes a single target with immense force.
//! The tremendous impact pins the target in place momentarily.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "crushing_blow",
    name = "Crushing Blow",
    tags = [Melee, SingleTarget],
    range = 3.8,
    geometry = cone(radius = 3.2, angle_deg = 35.0),
    potency = 210.0,
    cast_time = 0.5,
    cooldown = 4.5,
    energy_cost = 14.0,
    stun_seconds = 0.6,
    animation = "hammer_smash",
    impact_vfx = "crushing_blow_impact",
)]
pub struct CrushingBlow;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    CrushingBlow::register(registry);
}
