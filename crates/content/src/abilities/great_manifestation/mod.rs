//! Great Manifestation — Staff ultimate ability (E).
//!
//! Channels a massive arcane construct that devastates a large area with sustained damage.
//! The manifested energies burn with unnatural intensity.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "great_manifestation",
    name = "Great Manifestation",
    tags = [Ranged, Area, Ground],
    range = 22.0,
    geometry = circle(radius = 9.0),
    potency = 290.0,
    cast_time = 1.4,
    cooldown = 26.0,
    energy_cost = 48.0,
    statuses = [Burn],
    stun_seconds = 0.8,
    impact_delay = 0.4,
    animation = "staff_ultimate",
    impact_vfx = "great_manifestation_impact",
)]
pub struct GreatManifestation;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    GreatManifestation::register(registry);
}
