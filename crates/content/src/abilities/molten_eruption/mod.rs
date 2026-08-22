//! Molten Eruption — aerial self-centered slam.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "molten_eruption",
    name = "Molten Eruption",
    tags = [Area],
    range = 0.0,
    geometry = circle(radius = 8.0),
    potency = 50.0,
    cast_time = 0.0,
    cooldown = 14.0,
    mana_cost = 0.0,
    animation = "molten_eruption",
    impact_vfx = "molten_eruption_impact",
    icon = "",
)]
pub struct MoltenEruption;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    MoltenEruption::register(registry);
}
