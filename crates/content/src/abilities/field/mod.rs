//! Field — Focus secondary ability (W).
//!
//! Establishes an energy field in an area that damages enemies over time.
//! The field distorts space, slowing anyone who enters its bounds.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "field",
    name = "Field",
    tags = [Ranged, Area, PersistentCompatible],
    range = 14.0,
    geometry = circle(radius = 7.0),
    potency = 85.0,
    cast_time = 0.5,
    cooldown = 7.0,
    energy_cost = 18.0,
    statuses = [Slow],
    animation = "focus_field",
    impact_vfx = "field_impact",
)]
pub struct Field;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Field::register(registry);
}
