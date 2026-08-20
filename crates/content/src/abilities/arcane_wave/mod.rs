//! Arcane Wave — Staff secondary ability (W).
//!
//! Releases a broad wave of energy in a forward cone, damaging all enemies
//! caught in its path and leaving them sluggish as arcane residue clings to them.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "arcane_wave",
    name = "Arcane Wave",
    tags = [Ranged, Area],
    range = 16.0,
    geometry = cone(radius = 8.0, angle_deg = 55.0),
    potency = 125.0,
    cast_time = 0.4,
    cooldown = 5.5,
    energy_cost = 15.0,
    impact_delay = 0.15,
    animation = "staff_wave",
    impact_vfx = "arcane_wave_impact",
)]
pub struct ArcaneWave;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    ArcaneWave::register(registry);
}
