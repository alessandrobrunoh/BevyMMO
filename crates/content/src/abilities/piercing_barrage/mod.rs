//! Piercing Barrage — Bow ultimate ability (E).
//!
//! Unleashes a devastating stream of armor-piercing projectiles that
//! punch through multiple enemies in a line. The barrage staggers anyone caught.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "piercing_barrage",
    name = "Piercing Barrage",
    tags = [Ranged, Projectile, Area],
    range = 32.0,
    geometry = projectile(speed = 30.0),
    potency = 310.0,
    cast_time = 0.9,
    cooldown = 24.0,
    energy_cost = 42.0,
    statuses = [Stun],
    stun_seconds = 1.2,
    impact_delay = 0.25,
    animation = "bow_ultimate",
    impact_vfx = "piercing_barrage_impact",
)]
pub struct PiercingBarrage;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    PiercingBarrage::register(registry);
}
