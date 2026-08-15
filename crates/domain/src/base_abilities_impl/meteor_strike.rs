//! "Meteorite" — unica Ultimate della `MagicStaff` (uno slot Ultimate offre
//! sempre un solo gesto, non si sceglie).
//!
//! Due secondi di cerchio rosso a terra, poi lo schianto. Il preavviso lungo
//! è il prezzo del danno: chi lo subisce ha tutto il tempo di uscirne, e chi
//! lo lancia deve leggere dove sarà il bersaglio, non dov'è.

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "meteor_strike",
    name = "Meteorite",
    tags = [Ranged, Area, Ground, PersistentCompatible],
    geometry = circle(radius = 5.0, range = 16.0),
    power = 520.0,
    cast_time = 0.8,
    cooldown = 25.0,
    energy_cost = 35.0,
    animation = "staff_raise",
    impact_vfx = "meteor_strike_impact",
    impact_delay = 2.0,
)]
pub struct MeteorStrike;
