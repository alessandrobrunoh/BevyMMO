//! Ground Break — boots secondary ability.

use bevymmo_props_macro::base_ability;
use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "ground_break",
    name = "Ground Break",
    tags = [Melee, Area],
    range = 4.0,
    geometry = circle(radius = 3.0, range = 4.0),
    potency = 135.0,
    cast_time = 0.35,
    cooldown = 9.0,
    energy_cost = 16.0,
    statuses = [Root],
    animation = "boots_ground_break",
    impact_vfx = "ground_break_impact",
)]
pub struct GroundBreak;

pub fn register(registry: &mut BaseAbilityRegistry) { GroundBreak::register(registry); }
