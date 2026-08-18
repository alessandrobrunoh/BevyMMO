//! Domain — Focus ultimate ability (E).
//!
//! Claims a large area as your domain, exerting powerful control over
//! enemies caught within its bounds. The domain saps mobility and anchors foes.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "domain",
    name = "Domain",
    tags = [Ranged, Area, Ground, PersistentCompatible],
    range = 20.0,
    geometry = circle(radius = 12.0),
    potency = 210.0,
    cast_time = 1.1,
    cooldown = 26.0,
    energy_cost = 45.0,
    statuses = [Slow, Root],
    stun_seconds = 1.5,
    animation = "focus_ultimate",
    impact_vfx = "domain_impact",
)]
pub struct Domain;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Domain::register(registry);
}
