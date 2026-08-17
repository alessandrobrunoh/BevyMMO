//! Essenza Fuoco — §10 del design: "Lo Schianto infligge danno da Fuoco".
//! Bersaglio di default: nemici (§13, nessun Glifo "CHI" dedicato).

use bevymmo_props_macro::essence;

use crate::abilities::{AbilityParams, BaseAbility, EssenceEffect, EssenceRegistry};
use crate::spells::context::SpellCastContext;

#[essence(id = "fuoco", name = "Fuoco", rune_cost = 2, targets = enemies, color = (0.95, 0.35, 0.1))]
pub struct FuocoEssence;

/// Adds this content package to the essence registry.
pub fn register(registry: &mut EssenceRegistry) {
    FuocoEssence::register(registry);
}

impl FuocoEssence {
    /// Il Fuoco brucia più forte del semplice impatto fisico del gesto
    /// nudo — differenzia visibilmente "con Essenza" da "senza".
    pub const POWER_MULTIPLIER: f32 = 1.2;
}

impl EssenceEffect for FuocoEssence {
    fn manifest(
        &self,
        ability: &dyn BaseAbility,
        params: &AbilityParams,
        ctx: &mut SpellCastContext,
    ) {
        // La geometria (dove/quanto colpisce) è già del gesto; l'Essenza
        // decide solo cosa manifesta lì: danno da fuoco amplificato.
        ability.emit_damage_for_geometry(params.potency * Self::POWER_MULTIPLIER, params, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_stable() {
        assert_eq!(FuocoEssence::ID, "fuoco");
    }
}
