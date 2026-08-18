//! Power Shot — Bow primary ability (Q).
//!
//! Fires a heavily charged arrow that deals massive single-target damage.
//! Requires careful aim and a moment to fully draw the bowstring.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "power_shot",
    name = "Power Shot",
    tags = [Ranged, Projectile, SingleTarget, RepeatCompatible],
    range = 34.0,
    geometry = projectile(speed = 36.0),
    potency = 240.0,
    cast_time = 0.6,
    cooldown = 4.0,
    energy_cost = 12.0,
    impact_delay = 0.1,
    animation = "bow_draw",
    impact_vfx = "power_shot_impact",
)]
pub struct PowerShot;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    PowerShot::register(registry);
}
