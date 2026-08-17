//! Power Shot — Bow primary ability (Q).
//!
//! Fires a heavily charged arrow that deals massive single-target damage.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "power_shot",
    name = "Power Shot",
    tags = [Ranged, Projectile, SingleTarget, RepeatCompatible],
    range = 30.0,
    geometry = projectile(speed = 32.0),
    potency = 200.0,
    cast_time = 0.4,
    cooldown = 3.0,
    energy_cost = 10.0,
    animation = "bow_draw",
    impact_vfx = "power_shot_impact",
)]
pub struct PowerShot;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    PowerShot::register(registry);
}
