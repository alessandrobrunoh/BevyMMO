//! Astral Nova — area ultimate for staff weapons.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "astral_nova",
    name = "Nova Astrale",
    tags = [Ranged, Area],
    range = 18.0,
    geometry = circle(radius = 5.5, range = 18.0),
    potency = 480.0,
    cast_time = 1.2,
    cooldown = 24.0,
    energy_cost = 40.0,
    animation = "staff_nova_cast",
    impact_vfx = "astral_nova_impact",
)]
pub struct AstralNova;

pub fn register(registry: &mut BaseAbilityRegistry) {
    AstralNova::register(registry);
}
