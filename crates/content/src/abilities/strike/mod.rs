//! Strike — Gauntlets primary ability (Q).
//!
//! A lightning-fast flurry of punches that strikes a single target.
//! The rapid assault leaves the wielder momentarily hastened.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "strike",
    name = "Strike",
    tags = [Melee, SingleTarget, RepeatCompatible],
    range = 3.0,
    geometry = cone(radius = 2.8, angle_deg = 30.0),
    potency = 105.0,
    cast_time = 0.08,
    cooldown = 1.5,
    energy_cost = 5.0,
    statuses = [Stun],
    animation = "gauntlet_strike",
    impact_vfx = "strike_impact",
)]
pub struct Strike;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Strike::register(registry);
}
