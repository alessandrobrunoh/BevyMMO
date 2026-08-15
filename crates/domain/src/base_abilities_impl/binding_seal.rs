//! "Sigillo Vincolante" — prima opzione Secondary della `MagicStaff`.
//!
//! Cerchio a terra con mezzo secondo di preavviso, poi Stun a chi è dentro.
//! Il danno è volutamente basso: il valore del gesto è il controllo, non il
//! burst (chi vuole danno prende `ArcaneGale`).

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "binding_seal",
    name = "Sigillo Vincolante",
    tags = [Ranged, Area, Ground],
    geometry = circle(radius = 4.0, range = 12.0),
    power = 60.0,
    cast_time = 0.3,
    cooldown = 12.0,
    energy_cost = 18.0,
    animation = "staff_seal",
    impact_vfx = "binding_seal_impact",
    impact_delay = 0.5,
    stun_seconds = 1.5,
)]
pub struct BindingSeal;
