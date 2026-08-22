//! Wing Buffet — self-centered knockback ring.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "wing_buffet",
    name = "Wing Buffet",
    tags = [Melee, Area],
    range = 0.0,
    geometry = circle(radius = 6.0),
    potency = 31.0,
    cast_time = 0.0,
    cooldown = 10.0,
    mana_cost = 0.0,
    animation = "wing_buffet",
    impact_vfx = "wing_buffet_impact",
    icon = "",
)]
pub struct WingBuffet;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    WingBuffet::register(registry);
}
