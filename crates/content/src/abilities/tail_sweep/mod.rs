//! Tail Sweep — close-range self-centered ring.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "tail_sweep",
    name = "Tail Sweep",
    tags = [Melee, Area],
    range = 0.0,
    geometry = circle(radius = 5.0),
    potency = 34.0,
    cast_time = 0.0,
    cooldown = 7.0,
    mana_cost = 0.0,
    animation = "tail_sweep",
    impact_vfx = "tail_sweep_impact",
    icon = "",
)]
pub struct TailSweep;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    TailSweep::register(registry);
}
