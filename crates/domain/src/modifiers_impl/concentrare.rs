//! Modificatore Concentrare — §22 del design: opposto di Espandere. Riduce
//! l'area, aumenta l'intensità. Stesso requisito di tag di Espandere (`Area`
//! — un gesto single-target non ha un'area da concentrare).

use bevymmo_props_macro::modifier;

use crate::abilities::{AbilityParams, ModifierEffect};

#[modifier(id = "concentrare", name = "Concentrare", requires_tag = Area, rune_cost = 2)]
pub struct ConcentrareModifier;

impl ConcentrareModifier {
    pub const AREA_MULTIPLIER: f32 = 0.5;
    pub const POWER_MULTIPLIER: f32 = 1.7;
}

impl ModifierEffect for ConcentrareModifier {
    fn transform(&self, params: &mut AbilityParams) {
        params.area *= Self::AREA_MULTIPLIER;
        params.power *= Self::POWER_MULTIPLIER;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_shrinks_area_and_boosts_power() {
        let mut params = AbilityParams { power: 100.0, area: 4.0, range: 0.0, cast_time: 0.5, cooldown: 5.0, energy_cost: 10.0 };
        ConcentrareModifier.transform(&mut params);
        assert!((params.area - 2.0).abs() < 0.001);
        assert!((params.power - 170.0).abs() < 0.001);
    }
}
