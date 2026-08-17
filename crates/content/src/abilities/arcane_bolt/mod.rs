//! Arcane Bolt — Staff primary ability (Q).
//!
//! Fires a fast projectile that strikes the first target in a narrow frontal arc.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "arcane_bolt",
    name = "Arcane Bolt",
    tags = [Ranged, Projectile, SingleTarget, RepeatCompatible, EchoCompatible],
    range = 24.0,
    geometry = projectile(speed = 28.0),
    potency = 180.0,
    cast_time = 0.2,
    cooldown = 2.0,
    energy_cost = 8.0,
    animation = "staff_thrust",
    impact_vfx = "arcane_bolt_impact",
)]
pub struct ArcaneBolt;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    ArcaneBolt::register(registry);
}
