//! Molten Eruption spell implementation.
//!
//! 1.0s CastTime AoE that places 6 delayed fire circles in a ring around
//! the caster. Circles have staggered impact delays (0.0-0.75s) and deal
//! moderate damage.

use glam::Vec3;

use crate::spells::{AoeEffect, AoeTargeting, Spell, SpellCastContext, SpellConfig, SpellId};

pub struct MoltenEruptionSpell;

impl MoltenEruptionSpell {
    pub const ID: &'static str = "molten_eruption";
    pub const DISPLAY_NAME: &'static str = "Molten Eruption";
    pub const COOLDOWN_SECONDS: f32 = 8.0;
    pub const CAST_RANGE: f32 = 0.0;
    pub const AREA_RADIUS: f32 = 6.0;
    pub const CAST_TIME_SECONDS: f32 = 1.0;
    pub const CIRCLE_RADIUS: f32 = 2.5;
    pub const CIRCLE_COUNT: usize = 6;
    pub const RING_RADIUS: f32 = 6.0;
    pub const DAMAGE_MULTIPLIER: f32 = 2.0;
    pub const BASE_IMPACT_DELAY_SECONDS: f32 = 0.0;
    pub const STAGGER_INCREMENT_SECONDS: f32 = 0.15;

    /// Generates the center position for a circle at the given index.
    ///
    /// Circles are placed at 6 evenly spaced angles around the caster.
    pub fn circle_center(index: usize, caster_position: Vec3) -> Vec3 {
        let angle = (index as f32 / Self::CIRCLE_COUNT as f32) * std::f32::consts::TAU;
        let x = angle.cos() * Self::RING_RADIUS;
        let z = angle.sin() * Self::RING_RADIUS;
        caster_position + Vec3::new(x, 0.0, z)
    }

    /// Returns the staggered impact delay for a circle at the given index.
    pub fn impact_delay(index: usize) -> f32 {
        Self::BASE_IMPACT_DELAY_SECONDS + (index as f32 * Self::STAGGER_INCREMENT_SECONDS)
    }
}

impl Spell for MoltenEruptionSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::ranged_aoe(Self::COOLDOWN_SECONDS, Self::CAST_RANGE, Self::AREA_RADIUS)
            .with_cast_time(Self::CAST_TIME_SECONDS)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        // Emit 6 staggered circles around the caster
        for index in 0..Self::CIRCLE_COUNT {
            let center = Self::circle_center(index, ctx.caster_position);
            let delay = Self::impact_delay(index);

            ctx.emit_aoe_with_delay(
                center,
                Self::CIRCLE_RADIUS,
                delay,
                delay,
                Self::ID,
                AoeEffect::Damage {
                    amount: damage,
                    targeting: AoeTargeting::ExcludeCaster,
                },
            );
        }

        // Visual at caster center
        ctx.emit_visual(Self::ID, ctx.caster_position, ctx.caster_position);
    }
}

#[cfg(test)]
mod tests {
    use crate::EntityId;
    use super::*;
    use crate::spells::context::SpellCastContext;
    use crate::stats::components::CombatStats;

    #[test]
    fn molten_eruption_has_expected_id_and_config() {
        let spell = MoltenEruptionSpell;
        assert_eq!(spell.id().as_str(), "molten_eruption");
        assert_eq!(spell.display_name(), "Molten Eruption");
        assert_eq!(spell.config().cooldown_seconds, 8.0);
        assert_eq!(spell.config().cast_time_seconds, 1.0);
    }

    #[test]
    fn cast_emits_six_staggered_circles() {
        let caster = EntityId::new(1);

        let spell = MoltenEruptionSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_aoes.len(), 6);
        assert_eq!(ctx.pending_visuals.len(), 1);

        // Check staggered delays
        for (index, aoe) in ctx.pending_aoes.iter().enumerate() {
            let expected_delay = index as f32 * 0.15;
            assert!((aoe.initial_delay_seconds - expected_delay).abs() < 0.01);

            // Check damage amount
            if let AoeEffect::Damage { amount, .. } = &aoe.effect {
                assert_eq!(*amount, 100.0); // 50 * 2.0
            } else {
                panic!("Expected Damage effect");
            }
        }
    }

    #[test]
    fn circle_centers_form_ring_around_caster() {
        let caster_pos = Vec3::new(10.0, 0.0, 5.0);

        // First circle (0°)
        let center0 = MoltenEruptionSpell::circle_center(0, caster_pos);
        assert!((center0.x - (caster_pos.x + 6.0)).abs() < 0.01);
        assert!((center0.z - caster_pos.z).abs() < 0.01);

        // Third circle (180°)
        let center3 = MoltenEruptionSpell::circle_center(3, caster_pos);
        assert!((center3.x - (caster_pos.x - 6.0)).abs() < 0.01);
        assert!((center3.z - caster_pos.z).abs() < 0.01);

        // Sixth circle (300°)
        let center5 = MoltenEruptionSpell::circle_center(5, caster_pos);
        // cos(300°) ≈ 0.5, sin(300°) ≈ -0.866
        assert!((center5.x - (caster_pos.x + 3.0)).abs() < 0.1);
        assert!((center5.z - (caster_pos.z - 5.196)).abs() < 0.1);
    }

    #[test]
    fn impact_delays_stagger_correctly() {
        assert!((MoltenEruptionSpell::impact_delay(0) - 0.0).abs() < 0.001);
        assert!((MoltenEruptionSpell::impact_delay(1) - 0.15).abs() < 0.001);
        assert!((MoltenEruptionSpell::impact_delay(2) - 0.30).abs() < 0.001);
        assert!((MoltenEruptionSpell::impact_delay(3) - 0.45).abs() < 0.001);
        assert!((MoltenEruptionSpell::impact_delay(4) - 0.60).abs() < 0.001);
        assert!((MoltenEruptionSpell::impact_delay(5) - 0.75).abs() < 0.001);
    }
}
