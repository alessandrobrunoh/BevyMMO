//! "Sfera Arcana" — prima opzione Primary della `MagicStaff`.
//!
//! Lancia una palla davanti a sé: vola davvero (entità replicata, vedi
//! `SpellCastContext::emit_projectile`) e colpisce la prima entità nel
//! corridoio frontale, senza bisogno di selezionare un bersaglio.

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "arcane_orb",
    name = "Sfera Arcana",
    tags = [Ranged, Projectile, SingleTarget, RepeatCompatible],
    geometry = projectile(range = 22.0, speed = 24.0),
    power = 220.0,
    cast_time = 0.25,
    cooldown = 2.5,
    energy_cost = 10.0,
    animation = "staff_thrust",
    impact_vfx = "arcane_orb_impact",
)]
pub struct ArcaneOrb;
