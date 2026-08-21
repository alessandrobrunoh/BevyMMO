//! Searing Breath — frontal fire cone aimed at the current target.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "searing_breath",
    name = "Searing Breath",
    tags = [Area],
    range = 12.0,
    geometry = cone(radius = 10.0, angle_deg = 50.0),
    potency = 45.0,
    cast_time = 0.0,
    cooldown = 8.0,
    mana_cost = 0.0,
    animation = "searing_breath",
    impact_vfx = "searing_breath_impact",
    icon = "",
)]
pub struct SearingBreath;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    SearingBreath::register(registry);
}
