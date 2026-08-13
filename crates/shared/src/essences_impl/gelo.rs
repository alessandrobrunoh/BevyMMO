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

        match ability.geometry() {
            AbilityGeometry::Cone { radius, .. } | AbilityGeometry::Circle { radius } => {
                let center = ctx.effective_center();
                let area = params.area.max(radius);
                ctx.emit_aoe(
                    center,
                    area,
                    0.0,
                    ability.id().as_str().to_string(),
                    AoeEffect::Damage { amount: chilled_power, targeting: AoeTargeting::ExcludeCaster },
                );
                ctx.emit_aoe(
                    center,
                    area,
                    0.0,
                    ability.id().as_str().to_string(),
                    AoeEffect::ApplyModifier {
                        effects: Self::slow_effect(),
                        duration_seconds: Some(Self::SLOW_DURATION_SECONDS),
                        kind: ModifierKind::Debuff,
                        once_per_entity: true,
                        targeting: AoeTargeting::ExcludeCaster,
                    },
                );
                ctx.emit_visual(ability.impact_vfx().to_string(), center, center);
            }
            AbilityGeometry::Projectile { .. } => {
                if let Some(target) = ctx.target_entity {
                    ctx.emit_damage(target, chilled_power);
                    ctx.emit_modifier(target, Self::slow_effect(), Some(Self::SLOW_DURATION_SECONDS), ModifierKind::Debuff);
                    let target_position = ctx
                        .potential_targets
                        .iter()
                        .find(|(entity, _)| *entity == target)
                        .map(|(_, position)| *position)
                        .unwrap_or(ctx.caster_position);
                    ctx.emit_visual(ability.impact_vfx().to_string(), ctx.caster_position, target_position);
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
