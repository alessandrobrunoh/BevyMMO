//! "Scintilla" — alternativa più rapida e leggera a Getto per lo slot
//! Primary: meno potenza e cooldown più basso, stesso raggio.

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "staff_spark",
    name = "Scintilla",
    tags = [Ranged, Projectile, SingleTarget],
    geometry = projectile(range = 18.0, speed = 32.0),
    power = 140.0,
    cast_time = 0.15,
    cooldown = 1.5,
    energy_cost = 6.0,
    animation = "staff_spark_flick",
    impact_vfx = "spark_impact_burst",
)]
pub struct StaffSpark;
