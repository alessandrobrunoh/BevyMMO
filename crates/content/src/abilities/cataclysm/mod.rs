//! Cataclysm — berserk arena slam. Same id the dragon rotation already used.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "cataclysm",
    name = "Cataclysm",
    tags = [Area],
    range = 0.0,
    geometry = circle(radius = 10.0),
    potency = 62.0,
    cast_time = 0.0,
    cooldown = 20.0,
    mana_cost = 0.0,
    animation = "cataclysm",
    impact_vfx = "cataclysm_impact",
    icon = "",
)]
pub struct Cataclysm;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    Cataclysm::register(registry);
}
