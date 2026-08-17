//! Domain — Focus ultimate ability (E).
//!
//! Claims a large area as your domain, exerting powerful control over
//! enemies caught within its bounds.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "domain",
    name = "Domain",
    tags = [Ranged, Area, Ground, PersistentCompatible],
    range = 18.0,
    geometry = circle(radius = 10.0),
    potency = 240.0,
    cast_time = 1.0,
    cooldown = 24.0,
    energy_cost = 42.0,
    animation = "focus_ultimate",
    impact_vfx = "domain_impact",
)]
pub struct Domain;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Domain::register(registry);
}
