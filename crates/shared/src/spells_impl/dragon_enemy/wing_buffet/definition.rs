//! Wing Buffet spell implementation.
//!
//! Instant expanding ring attack from caster (radius 10.0) that applies
//! moderate damage and a 0.4s Stun to all enemies in range.

use crate::crowd_control::CrowdControlKind;
use crate::spells::{AoeEffect, AoeTargeting, Spell, SpellCastContext, SpellConfig, SpellId};

pub struct WingBuffetSpell;

impl WingBuffetSpell {
    pub const ID: &'static str = "wing_buffet";
    pub const DISPLAY_NAME: &'static str = "Wing Buffet";
    pub const COOLDOWN_SECONDS: f32 = 12.0;
    pub const RADIUS: f32 = 10.0;
    pub const DAMAGE_MULTIPLIER: f32 = 0.8;
    pub const STUN_DURATION_SECONDS: f32 = 0.4;
    pub const AOE_DURATION_SECONDS: f32 = 0.1;
}

impl Spell for WingBuffetSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::ranged_aoe(Self::COOLDOWN_SECONDS, 10.0, Self::RADIUS)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        // Emit AoE Stun at caster center
        ctx.emit_aoe(
            ctx.caster_position,
            Self::RADIUS,
            Self::AOE_DURATION_SECONDS,
            Self::ID,
            AoeEffect::CrowdControl {
                kind: CrowdControlKind::Stun,
                duration_seconds: Self::STUN_DURATION_SECONDS,
                once_per_entity: true,
                targeting: AoeTargeting::ExcludeCaster,
            },
        );

        // Emit AoE Damage at same center and radius
        ctx.emit_aoe(
            ctx.caster_position,
            Self::RADIUS,
            Self::AOE_DURATION_SECONDS,
            Self::ID,
            AoeEffect::Damage {
                amount: damage,
                targeting: AoeTargeting::ExcludeCaster,
            },
        );

        ctx.emit_visual(Self::ID, ctx.caster_position, ctx.caster_position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spells::context::SpellCastContext;
    use crate::stats::components::CombatStats;
    use bevy::prelude::{Vec3, World};

    #[test]
    fn wing_buffet_has_expected_id_and_config() {
        let spell = WingBuffetSpell;
        assert_eq!(spell.id().as_str(), "wing_buffet");
        assert_eq!(spell.display_name(), "Wing Buffet");
        assert_eq!(spell.config().cooldown_seconds, 12.0);
    }

    #[test]
    fn cast_emits_aoe_stun_and_damage() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();

        let spell = WingBuffetSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_aoes.len(), 2);
        assert_eq!(ctx.pending_visuals.len(), 1);

        // First should be stun
        if let AoeEffect::CrowdControl { kind, .. } = &ctx.pending_aoes[0].effect {
            assert_eq!(*kind, CrowdControlKind::Stun);
        } else {
            panic!("First effect should be CrowdControl");
        }

        // Second should be damage
        if let AoeEffect::Damage { amount, .. } = &ctx.pending_aoes[1].effect {
            assert_eq!(*amount, 40.0);
        } else {
            panic!("Second effect should be Damage");
        }
    }
}
