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

        // Forma, ritardo e aggancio restano del gesto; la Terra aggiunge lo
        // Stagger sopra la stessa area.
        ability.emit_damage_for_geometry(earthen_power, params, ctx);

        if matches!(
            ability.geometry(),
            AbilityGeometry::Cone { .. } | AbilityGeometry::Circle { .. }
        ) {
            ability.emit_area_effect(
                params,
                ctx,
                AoeEffect::CrowdControl {
                    kind: CrowdControlKind::Stun,
                    duration_seconds: Self::STAGGER_DURATION_SECONDS,
                    once_per_entity: true,
                    targeting: AoeTargeting::ExcludeCaster,
                },
            );
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
