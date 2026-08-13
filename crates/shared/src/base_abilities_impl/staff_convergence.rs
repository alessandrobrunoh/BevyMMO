//! "Convergenza" — Ultimate: cerchio ampio, alto costo, alta capacità di
//! personalizzazione runica (più lenta e rara, quindi bilanciabile anche
//! con incisioni pesanti — vedi §34 del design).

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "staff_convergence",
    name = "Convergenza",
    tags = [Ranged, Area, Ground, PersistentCompatible, EchoCompatible, RepeatCompatible],
    geometry = circle(radius = 5.0),
    power = 520.0,
    cast_time = 2.4,
    cooldown = 45.0,
    energy_cost = 55.0,
    animation = "staff_convergence_channel",
    impact_vfx = "arcane_convergence_burst",
)]
pub struct StaffConvergence;
