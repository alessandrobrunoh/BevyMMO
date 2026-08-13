//! Modificatore Espandere — §19-20 del design: aumenta l'area di una spell
//! che possiede già un'area, a costo di velocità/efficienza. Richiede il tag
//! `Area` sulla `BaseAbility` (una spell single-target non può "espandersi").

use bevymmo_props_macro::modifier;

use crate::abilities::{AbilityParams, ModifierEffect};

#[modifier(id = "espandere", name = "Espandere", requires_tag = Area, rune_cost = 2)]
pub struct EspandereModifier;

impl EspandereModifier {
    pub const AREA_MULTIPLIER: f32 = 1.35;
    pub const CAST_TIME_BONUS_SECONDS: f32 = 0.2;
    pub const ENERGY_COST_MULTIPLIER: f32 = 1.2;
}

impl ModifierEffect for EspandereModifier {
    fn transform(&self, params: &mut AbilityParams) {
        params.area *= Self::AREA_MULTIPLIER;
        params.cast_time += Self::CAST_TIME_BONUS_SECONDS;
        params.energy_cost *= Self::ENERGY_COST_MULTIPLIER;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_grows_area_at_the_cost_of_cast_time_and_energy() {
        let mut params = AbilityParams { power: 100.0, area: 4.0, range: 0.0, cast_time: 0.5, cooldown: 5.0, energy_cost: 10.0 };
        EspandereModifier.transform(&mut params);
        assert!((params.area - 5.4).abs() < 0.001);
        assert!((params.cast_time - 0.7).abs() < 0.001);
        assert!((params.energy_cost - 12.0).abs() < 0.001);
    }
}
