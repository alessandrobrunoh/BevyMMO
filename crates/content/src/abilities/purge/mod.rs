//! "Purge" — self-targeted buff removal ability.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "purge",
    name = "Purge",
    tags = [SelfTarget],
    range = 0.0,
    geometry = self_buff(duration_seconds = 0.0),
    potency = 0.0,
    cast_time = 0.0,
    cooldown = 18.0,
    energy_cost = 20.0,
    animation = "purge",
    impact_vfx = "purge_impact",
)]
pub struct PurgeAbility;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    PurgeAbility::register(registry);
}
