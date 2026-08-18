//! Volley — Bow secondary ability (W).
//!
//! Looses a rapid spread of arrows in a very wide cone.
//! Each arrow deals modest damage but the cumulative effect is significant.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "volley",
    name = "Volley",
    tags = [Ranged, Area, Projectile],
    range = 20.0,
    geometry = cone(radius = 10.0, angle_deg = 70.0),
    potency = 95.0,
    cast_time = 0.3,
    cooldown = 6.0,
    energy_cost = 16.0,
    animation = "bow_volley",
    impact_vfx = "volley_impact",
)]
pub struct Volley;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Volley::register(registry);
}
