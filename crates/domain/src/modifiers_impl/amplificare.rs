//! Modificatore Amplificare — §24 del design: più potenza, a costo di
//! energia e cooldown. Ogni `Modifier` richiede ESATTAMENTE un tag (nessuna
//! opzione "qualunque gesto" nel modello attuale): `Ranged` è il tag più
//! generico disponibile oggi — tutti e tre i gesti dello Staff lo hanno, a
//! differenza di Espandere/Concentrare che hanno senso solo su gesti ad area.

use bevymmo_props_macro::modifier;

use crate::abilities::{AbilityParams, ModifierEffect};

#[modifier(id = "amplificare", name = "Amplificare", requires_tag = Ranged, rune_cost = 3)]
pub struct AmplificareModifier;

impl AmplificareModifier {
    pub const POWER_MULTIPLIER: f32 = 1.25;
    pub const ENERGY_COST_MULTIPLIER: f32 = 1.3;
    pub const COOLDOWN_MULTIPLIER: f32 = 1.15;
}

impl ModifierEffect for AmplificareModifier {
    fn transform(&self, params: &mut AbilityParams) {
        params.power *= Self::POWER_MULTIPLIER;
        params.energy_cost *= Self::ENERGY_COST_MULTIPLIER;
        params.cooldown *= Self::COOLDOWN_MULTIPLIER;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_boosts_power_at_the_cost_of_energy_and_cooldown() {
        let mut params = AbilityParams { power: 100.0, area: 4.0, range: 0.0, cast_time: 0.5, cooldown: 5.0, energy_cost: 10.0 };
        AmplificareModifier.transform(&mut params);
        assert!((params.power - 125.0).abs() < 0.001);
        assert!((params.energy_cost - 13.0).abs() < 0.001);
        assert!((params.cooldown - 5.75).abs() < 0.001);
    }
}
