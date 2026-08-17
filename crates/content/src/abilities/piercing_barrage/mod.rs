//! Piercing Barrage — Bow ultimate ability (E).
//!
//! Unleashes a devastating stream of armor-piercing projectiles that
//! punch through multiple enemies in a line.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "piercing_barrage",
    name = "Piercing Barrage",
    tags = [Ranged, Projectile, Area],
    range = 28.0,
    geometry = projectile(speed = 26.0),
    potency = 280.0,
    cast_time = 0.8,
    cooldown = 22.0,
    energy_cost = 40.0,
    animation = "bow_ultimate",
    impact_vfx = "piercing_barrage_impact",
)]
pub struct PiercingBarrage;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    PiercingBarrage::register(registry);
}
