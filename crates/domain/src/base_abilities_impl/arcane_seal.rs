//! "Sigillo Arcano" — seconda opzione Primary della `MagicStaff`.
//!
//! Piazza un cerchio d'area dove punta il mouse (clampato a `range`), che
//! esplode subito. Alternativa a `ArcaneOrb`: meno danno sul singolo, ma
//! colpisce gruppi e non ha bisogno di allineare nessuno davanti a sé.

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "arcane_seal",
    name = "Sigillo Arcano",
    tags = [Ranged, Area, Ground, RepeatCompatible],
    geometry = circle(radius = 3.0, range = 14.0),
    power = 150.0,
    cast_time = 0.3,
    cooldown = 3.5,
    energy_cost = 12.0,
    animation = "staff_slam",
    impact_vfx = "arcane_seal_impact",
)]
pub struct ArcaneSeal;
