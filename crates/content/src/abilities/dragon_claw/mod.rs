//! Dragon Claw — melee cone, the grounded auto-attack.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "dragon_claw",
    name = "Dragon Claw",
    tags = [Melee, Area],
    range = 4.0,
    geometry = cone(radius = 4.0, angle_deg = 40.0),
    potency = 40.0,
    cast_time = 0.0,
    cooldown = 4.0,
    mana_cost = 0.0,
    animation = "dragon_claw",
    impact_vfx = "dragon_claw_impact",
    icon = "",
)]
pub struct DragonClaw;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    DragonClaw::register(registry);
}
