//! Arcane Bolt — Staff primary ability (Q).
//!
//! Fires a fast projectile that strikes the first target in a narrow frontal arc.
//! The swift invocation leaves the caster momentarily hastened.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "arcane_bolt",
    name = "Arcane Bolt",
    tags = [Ranged, Projectile, SingleTarget, RepeatCompatible, EchoCompatible],
    range = 26.0,
    geometry = projectile(speed = 32.0),
    potency = 165.0,
    cast_time = 0.15,
    cooldown = 1.6,
    energy_cost = 7.0,
    statuses = [Burn],
    animation = "staff_thrust",
    impact_vfx = "arcane_bolt_impact",
)]
pub struct ArcaneBolt;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    ArcaneBolt::register(registry);
}
