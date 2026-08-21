//! Cinder Storm — ground circle dropped on the densest player cluster.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "cinder_storm",
    name = "Cinder Storm",
    tags = [Area, Ground],
    range = 16.0,
    geometry = circle(radius = 5.0),
    potency = 36.0,
    cast_time = 0.0,
    cooldown = 12.0,
    mana_cost = 0.0,
    animation = "cinder_storm",
    impact_vfx = "cinder_storm_impact",
    icon = "",
)]
pub struct CinderStorm;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    CinderStorm::register(registry);
}
