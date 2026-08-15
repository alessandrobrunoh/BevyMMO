//! Cataclysm spell implementation.
//!
//! Channeling arena-wide spell (5.0s duration). Every 0.5s tick emits
/// heavy AoE damage to all entities in radius 14.0. Each tick also spawns
/// an arena-wide red flash visual.
use crate::spells::{
    AoeEffect, AoeTargeting, ChannelMovementPolicy, Spell, SpellCastContext, SpellConfig, SpellId,
};

pub struct CataclysmSpell;

impl CataclysmSpell {
    pub const ID: &'static str = "cataclysm";
    pub const DISPLAY_NAME: &'static str = "Cataclysm";
    pub const COOLDOWN_SECONDS: f32 = 30.0;
    pub const CAST_RANGE: f32 = 0.0;
    pub const AREA_RADIUS: f32 = 14.0;
    pub const CHANNEL_DURATION_SECONDS: f32 = 5.0;
    pub const TICK_INTERVAL_SECONDS: f32 = 0.5;
    pub const DAMAGE_MULTIPLIER: f32 = 1.5;
    pub const AOE_DURATION_SECONDS: f32 = 0.2;
}

impl Spell for CataclysmSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::ranged_aoe(Self::COOLDOWN_SECONDS, Self::CAST_RANGE, Self::AREA_RADIUS)
            .with_channel(ChannelMovementPolicy::InterruptOnMove)
            .with_channel_duration(Self::CHANNEL_DURATION_SECONDS)
    }

    fn channel_tick_interval_seconds(&self) -> f32 {
        Self::TICK_INTERVAL_SECONDS
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        // Emit AoE damage to all entities in arena radius
        ctx.emit_aoe(
            ctx.caster_position,
            Self::AREA_RADIUS,
            Self::AOE_DURATION_SECONDS,
            Self::ID,
            AoeEffect::Damage {
                amount: damage,
                targeting: AoeTargeting::ExcludeCaster,
            },
        );

        // Visual each tick (arena-wide red flash)
        ctx.emit_visual(Self::ID, ctx.caster_position, ctx.caster_position);
    }
}

#[cfg(test)]
mod tests {
    use crate::EntityId;
    use super::*;
    use crate::spells::context::SpellCastContext;
    use crate::stats::components::CombatStats;
    use glam::Vec3;

    #[test]
    fn cataclysm_has_expected_id_and_config() {
        let spell = CataclysmSpell;
        assert_eq!(spell.id().as_str(), "cataclysm");
        assert_eq!(spell.display_name(), "Cataclysm");
        assert_eq!(spell.config().cooldown_seconds, 30.0);
        assert_eq!(spell.config().channel_duration_seconds, Some(5.0));
        assert!(spell.config().is_channel);
    }

    #[test]
    fn cast_emits_aoe_damage_and_visual_per_tick() {
        let caster = EntityId::new(1);

        let spell = CataclysmSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_aoes.len(), 1);
        assert_eq!(ctx.pending_visuals.len(), 1);

        // Check AoE damage
        if let AoeEffect::Damage { amount, .. } = &ctx.pending_aoes[0].effect {
            assert_eq!(*amount, 75.0); // 50 * 1.5
        } else {
            panic!("Expected Damage effect");
        }
    }

    #[test]
    fn channel_tick_interval_is_0_5_seconds() {
        let spell = CataclysmSpell;
        assert_eq!(spell.channel_tick_interval_seconds(), 0.5);
    }
}
