//! Field — Focus secondary ability (W).
//!
//! Establishes an energy field in an area that damages enemies over time.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "field",
    name = "Field",
    tags = [Ranged, Area, PersistentCompatible],
    range = 16.0,
    geometry = circle(radius = 6.0),
    potency = 100.0,
    cast_time = 0.4,
    cooldown = 6.0,
    energy_cost = 16.0,
    animation = "focus_field",
    impact_vfx = "field_impact",
)]
pub struct Field;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Field::register(registry);
}
