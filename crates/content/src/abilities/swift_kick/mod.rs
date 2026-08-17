//! Swift Kick — boots primary ability.

use bevymmo_props_macro::base_ability;
use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "swift_kick",
    name = "Swift Kick",
    tags = [Melee, SingleTarget],
    range = 4.0,
    geometry = cone(radius = 4.0, angle_deg = 65.0),
    potency = 85.0,
    cast_time = 0.0,
    cooldown = 4.0,
    energy_cost = 7.0,
    statuses = [Slow],
    animation = "boots_swift_kick",
    impact_vfx = "swift_kick_impact",
)]
pub struct SwiftKick;

pub fn register(registry: &mut BaseAbilityRegistry) { SwiftKick::register(registry); }
