//! Ground Slam — Hammer secondary ability (W).
//!
//! Slams the ground with tremendous force, creating a shockwave that
//! damages and destabilizes nearby enemies. The tremor leaves foes sluggish.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "ground_slam",
    name = "Ground Slam",
    tags = [Melee, Area, Ground],
    range = 5.0,
    geometry = circle(radius = 5.0),
    potency = 140.0,
    cast_time = 0.5,
    cooldown = 8.5,
    energy_cost = 22.0,
    stun_seconds = 1.0,
    impact_delay = 0.2,
    animation = "hammer_slam",
    impact_vfx = "ground_slam_impact",
)]
pub struct GroundSlam;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    GroundSlam::register(registry);
}
