//! Mind Ward — helmet secondary cleanse ability.

use crate::abilities::BaseAbilityRegistry;
use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "mind_ward",
    name = "Mind Ward",
    tags = [SelfTarget],
    range = 0.0,
    geometry = self_buff(duration_seconds = 0.0),
    potency = 0.0,
    cast_time = 0.0,
    cooldown = 8.0,
    energy_cost = 14.0,
    cleanse = Debuffs,
    animation = "helmet_mind_ward",
    impact_vfx = "mind_ward_impact",
)]
pub struct MindWard;

pub fn register(registry: &mut BaseAbilityRegistry) {
    MindWard::register(registry);
}
