//! Essenza Gelo — §11 del design: "I nemici colpiti vengono rallentati."
//! Meno danno di Fuoco (punta sul controllo, non sul burst).

use bevymmo_props_macro::essence;

use crate::abilities::{AbilityGeometry, AbilityParams, BaseAbility, EssenceEffect};
use crate::spells::context::{AoeEffect, AoeTargeting, SpellCastContext};
use crate::stats::events::{ModifierEffect, ModifierKind, ModifierOp, StatField};

#[essence(id = "gelo", name = "Gelo", rune_cost = 2, targets = enemies, color = (0.55, 0.85, 1.0))]
pub struct GeloEssence;

impl GeloEssence {
    pub const POWER_MULTIPLIER: f32 = 0.7;
    pub const SLOW_MULTIPLIER: f32 = 0.5;
    pub const SLOW_DURATION_SECONDS: f32 = 3.0;

    fn slow_effect() -> Vec<ModifierEffect> {
        vec![ModifierEffect::Stat {
            field: StatField::Speed,
            operation: ModifierOp::Multiply,
            value: Self::SLOW_MULTIPLIER,
        }]
    }
}

impl EssenceEffect for GeloEssence {
    fn manifest(&self, ability: &dyn BaseAbility, params: &AbilityParams, ctx: &mut SpellCastContext) {
        let chilled_power = params.power * Self::POWER_MULTIPLIER;

        // Danno e forma restano quelli del gesto (compresi ritardo d'impatto
        // e auto-aggancio frontale del proiettile): il Gelo aggiunge solo il
        // rallentamento sopra la stessa geometria.
        ability.emit_damage_for_geometry(chilled_power, params, ctx);

        match ability.geometry() {
            AbilityGeometry::Cone { .. } | AbilityGeometry::Circle { .. } => {
                ability.emit_area_effect(
                    params,
                    ctx,
                    AoeEffect::ApplyModifier {
                        effects: Self::slow_effect(),
                        duration_seconds: Some(Self::SLOW_DURATION_SECONDS),
                        kind: ModifierKind::Debuff,
                        once_per_entity: true,
                        targeting: AoeTargeting::ExcludeCaster,
                    },
                );
            }
            AbilityGeometry::Projectile { .. } => {
                if let Some(target) = ability.projectile_target(ctx) {
                    ctx.emit_modifier(target, Self::slow_effect(), Some(Self::SLOW_DURATION_SECONDS), ModifierKind::Debuff);
                }
            }
            AbilityGeometry::SelfBuff { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_stable() {
        assert_eq!(GeloEssence::ID, "gelo");
    }
}
