//! "Onda" — cono ravvicinato. Gesto base di uno staff.

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "staff_wave",
    name = "Onda",
    tags = [Ranged, Area, Ground, RepeatCompatible],
    geometry = cone(radius = 6.0, angle_deg = 70.0),
    power = 190.0,
    cast_time = 0.5,
    cooldown = 8.0,
    energy_cost = 18.0,
    animation = "staff_wave_sweep",
    impact_vfx = "ground_wave_cone",
)]
pub struct StaffWave;
