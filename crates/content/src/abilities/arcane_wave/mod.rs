//! Arcane Wave — Staff secondary ability (W).
//!
//! Releases a wave of energy in a forward cone, damaging all enemies caught in its path.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "arcane_wave",
    name = "Arcane Wave",
    tags = [Ranged, Area],
    range = 18.0,
    geometry = cone(radius = 6.0, angle_deg = 45.0),
    potency = 140.0,
    cast_time = 0.35,
    cooldown = 5.0,
    energy_cost = 14.0,
    animation = "staff_wave",
    impact_vfx = "arcane_wave_impact",
)]
pub struct ArcaneWave;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    ArcaneWave::register(registry);
}
