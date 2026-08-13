//! "Nova" — alternativa più stretta e rapida a Onda per lo slot Secondary:
//! cerchio invece di cono, raggio minore, cooldown più basso.

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "staff_nova",
    name = "Nova",
    tags = [Ranged, Area, Ground, RepeatCompatible],
    geometry = circle(radius = 3.5),
    power = 150.0,
    cast_time = 0.3,
    cooldown = 5.0,
    energy_cost = 14.0,
    animation = "staff_nova_pulse",
    impact_vfx = "nova_impact_ring",
)]
pub struct StaffNova;
