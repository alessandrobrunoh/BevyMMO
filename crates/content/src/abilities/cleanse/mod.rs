//! "Cleanse" — self-targeted debuff removal ability.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "cleanse",
    name = "Cleanse",
    tags = [SelfTarget],
    range = 0.0,
    geometry = self_buff(duration_seconds = 0.0),
    potency = 0.0,
    cast_time = 0.0,
    cooldown = 12.0,
    energy_cost = 15.0,
    animation = "cleanse",
    impact_vfx = "cleanse_impact",
)]
pub struct CleanseAbility;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    CleanseAbility::register(registry);
}
