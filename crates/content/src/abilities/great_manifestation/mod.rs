//! Great Manifestation — Staff ultimate ability (E).
//!
//! Channels a massive arcane construct that devastates a large area with sustained damage.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "great_manifestation",
    name = "Great Manifestation",
    tags = [Ranged, Area, Ground],
    range = 20.0,
    geometry = circle(radius = 8.0),
    potency = 320.0,
    cast_time = 1.2,
    cooldown = 25.0,
    energy_cost = 45.0,
    animation = "staff_ultimate",
    impact_vfx = "great_manifestation_impact",
)]
pub struct GreatManifestation;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    GreatManifestation::register(registry);
}
