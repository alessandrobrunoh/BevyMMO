//! Volley — Bow secondary ability (W).
//!
//! Looses a spread of arrows in a wide cone, damaging all enemies in the area.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "volley",
    name = "Volley",
    tags = [Ranged, Area, Projectile],
    range = 22.0,
    geometry = cone(radius = 8.0, angle_deg = 60.0),
    potency = 120.0,
    cast_time = 0.5,
    cooldown = 7.0,
    energy_cost = 18.0,
    animation = "bow_volley",
    impact_vfx = "volley_impact",
)]
pub struct Volley;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Volley::register(registry);
}
