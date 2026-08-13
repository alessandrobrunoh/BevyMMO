//! "Getto" — colpo a distanza singolo bersaglio. Gesto base di uno staff.

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "staff_bolt",
    name = "Getto",
    tags = [Ranged, Projectile, SingleTarget],
    geometry = projectile(range = 20.0, speed = 26.0),
    power = 260.0,
    cast_time = 0.35,
    cooldown = 4.0,
    energy_cost = 12.0,
    animation = "staff_bolt_cast",
    impact_vfx = "bolt_impact_burst",
)]
pub struct StaffBolt;
