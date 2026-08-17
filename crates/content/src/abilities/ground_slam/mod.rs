//! Ground Slam — Hammer secondary ability (W).
//!
//! Slams the ground with tremendous force, creating a shockwave that
//! damages and destabilizes nearby enemies.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "ground_slam",
    name = "Ground Slam",
    tags = [Melee, Area, Ground],
    range = 4.5,
    geometry = circle(radius = 4.5),
    potency = 150.0,
    cast_time = 0.45,
    cooldown = 8.0,
    energy_cost = 20.0,
    statuses = [Slow],
    animation = "hammer_slam",
    impact_vfx = "ground_slam_impact",
)]
pub struct GroundSlam;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    GroundSlam::register(registry);
}
