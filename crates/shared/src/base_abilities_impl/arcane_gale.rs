//! "Raffica Arcana" — seconda opzione Secondary della `MagicStaff`.
//!
//! Cono davanti a sé, immediato: il rovescio esatto di `BindingSeal` (niente
//! controllo, niente preavviso, tutto danno ravvicinato) così la scelta fra
//! le due opzioni Secondary è una scelta vera.

use bevymmo_props_macro::base_ability;

#[base_ability(
    id = "arcane_gale",
    name = "Raffica Arcana",
    tags = [Ranged, Area, RepeatCompatible],
    geometry = cone(radius = 7.0, angle_deg = 75.0),
    power = 190.0,
    cast_time = 0.35,
    cooldown = 7.0,
    energy_cost = 16.0,
    animation = "staff_sweep",
    impact_vfx = "arcane_gale_impact",
)]
pub struct ArcaneGale;
