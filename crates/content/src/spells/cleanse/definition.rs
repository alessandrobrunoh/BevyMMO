//! Self-targeted debuff removal spell.

use bevymmo_props_macro::spell;
use bevymmo_gameplay::effects::{StatusFilter, StatusSelection};

use crate::spells::{SpellCast, SpellCastContext};

#[spell(
    id = "cleanse",
    name = "Cleanse",
    cooldown = 12.0,
    targeting = SelfCentered,
    range = 0.0,
    area = 0.0,
)]
pub struct CleanseSpell;

impl SpellCast for CleanseSpell {
    fn cast(&self, ctx: &mut SpellCastContext) {
        ctx.emit_cleanse(
            ctx.caster,
            bevymmo_gameplay::effects::CleanseEffect {
                filter: StatusFilter::Debuffs,
                max_statuses: Some(2),
                selection: StatusSelection::Oldest,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::effects::EffectSpec;
    use bevymmo_gameplay::stats::components::CombatStats;
    use bevymmo_gameplay::EntityId;
    use glam::Vec3;

    #[test]
    fn cleanse_targets_the_caster_and_removes_debuffs() {
        let caster = EntityId::new(1);
        let combat = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
        let mut context =
            SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        CleanseSpell.cast(&mut context);

        assert_eq!(context.pending_effects.len(), 1);
        assert_eq!(context.pending_effects[0].context.target, caster);
        assert!(matches!(
            context.pending_effects[0].effects[0],
            EffectSpec::Cleanse(_)
        ));
    }
}
