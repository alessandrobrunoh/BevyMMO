//! Essenza Terra — §12 del design: "I nemici subiscono forte Stagger."
//! Buona per Tank/frontline/CC (§12). Lo Stagger (via `CrowdControlKind::Stun`)
//! si applica solo sui gesti ad area (Cone/Circle): non esiste ancora un
//! percorso di CC single-target in `SpellCastContext`, quindi un gesto
//! Projectile con Terra incisa infligge solo danno bonus.

use bevymmo_props_macro::essence;

use crate::abilities::{AbilityGeometry, AbilityParams, BaseAbility, EssenceEffect};
use crate::crowd_control::CrowdControlKind;
use crate::spells::context::{AoeEffect, AoeTargeting, SpellCastContext};

#[essence(id = "terra", name = "Terra", rune_cost = 2, targets = enemies, color = (0.6, 0.45, 0.25))]
pub struct TerraEssence;

impl TerraEssence {
    pub const POWER_MULTIPLIER: f32 = 0.85;
    pub const STAGGER_DURATION_SECONDS: f32 = 1.2;
}

impl EssenceEffect for TerraEssence {
    fn manifest(&self, ability: &dyn BaseAbility, params: &AbilityParams, ctx: &mut SpellCastContext) {
        let earthen_power = params.power * Self::POWER_MULTIPLIER;

        match ability.geometry() {
            AbilityGeometry::Cone { radius, .. } | AbilityGeometry::Circle { radius } => {
                let center = ctx.effective_center();
                let area = params.area.max(radius);
                ctx.emit_aoe(
                    center,
                    area,
                    0.0,
                    ability.id().as_str().to_string(),
                    AoeEffect::Damage { amount: earthen_power, targeting: AoeTargeting::ExcludeCaster },
                );
                ctx.emit_aoe(
                    center,
                    area,
                    0.0,
                    ability.id().as_str().to_string(),
                    AoeEffect::CrowdControl {
                        kind: CrowdControlKind::Stun,
                        duration_seconds: Self::STAGGER_DURATION_SECONDS,
                        once_per_entity: true,
                        targeting: AoeTargeting::ExcludeCaster,
                    },
                );
                ctx.emit_visual(ability.impact_vfx().to_string(), center, center);
            }
            AbilityGeometry::Projectile { .. } => {
                ability.emit_damage_for_geometry(earthen_power, params, ctx);
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
        assert_eq!(TerraEssence::ID, "terra");
    }
}
