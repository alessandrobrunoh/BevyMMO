//! Tail Sweep spell implementation.
//!
//! Instant rear 180° cone attack (range 6.0) that applies moderate damage
//! and a 0.6s Stun. Only affects targets behind the caster: dot product
//! with caster_look_direction < -0.3.

use bevy::prelude::Vec3;

use crate::crowd_control::CrowdControlKind;
use crate::spells::{AoeEffect, AoeTargeting, Spell, SpellCastContext, SpellConfig, SpellId};

pub struct TailSweepSpell;

impl TailSweepSpell {
    pub const ID: &'static str = "tail_sweep";
    pub const DISPLAY_NAME: &'static str = "Tail Sweep";
    pub const COOLDOWN_SECONDS: f32 = 8.0;
    pub const RANGE: f32 = 6.0;
    pub const CONE_ANGLE_COS_THRESHOLD: f32 = -0.3; // Behind the caster
    pub const DAMAGE_MULTIPLIER: f32 = 0.6;
    pub const STUN_DURATION_SECONDS: f32 = 0.6;
    pub const STUN_RADIUS: f32 = 6.0;

    /// Filters targets that are behind the caster (rear 180° cone).
    fn is_behind_caster(caster_look: Vec3, to_target: Vec3) -> bool {
        let forward_flat = Vec3::new(caster_look.x, 0.0, caster_look.z).normalize();
        let to_target_flat = Vec3::new(to_target.x, 0.0, to_target.z);
        forward_flat.dot(to_target_flat) < Self::CONE_ANGLE_COS_THRESHOLD
    }
}

impl Spell for TailSweepSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::melee_aoe(Self::COOLDOWN_SECONDS, Self::RANGE)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        for &(target, target_pos) in ctx.potential_targets {
            if target == ctx.caster {
                continue;
            }

            let to_target = target_pos - ctx.caster_position;
            let distance = to_target.length();

            if distance > Self::RANGE {
                continue;
            }

            if !Self::is_behind_caster(ctx.caster_look_direction, to_target) {
                continue;
            }

            ctx.emit_damage(target, damage);
        }

        // Apply Stun via AoE at caster center (radius 6.0).
        ctx.emit_aoe(
            ctx.caster_position,
            Self::STUN_RADIUS,
            0.6,
            Self::ID,
            AoeEffect::CrowdControl {
                kind: CrowdControlKind::Stun,
                duration_seconds: Self::STUN_DURATION_SECONDS,
                once_per_entity: true,
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
    use bevy::prelude::World;

    #[test]
    fn tail_sweep_has_expected_id_and_config() {
        let spell = TailSweepSpell;
        assert_eq!(spell.id().as_str(), "tail_sweep");
        assert_eq!(spell.display_name(), "Tail Sweep");
        assert_eq!(spell.config().cooldown_seconds, 8.0);
    }

    #[test]
    fn cast_emits_damage_to_targets_behind_caster() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let target_behind = world.spawn_empty().id();
        let target_front = world.spawn_empty().id();

        let spell = TailSweepSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        // Caster facing +Z, target at -Z (behind).
        let potential = vec![
            (target_behind, Vec3::new(0.0, 0.0, -4.0)),
            (target_front, Vec3::new(0.0, 0.0, 4.0)),
        ];

        let mut ctx =
            SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &potential);

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_damage.len(), 1);
        assert_eq!(ctx.pending_damage[0].target, target_behind);
        assert!((ctx.pending_damage[0].amount - 30.0).abs() < 0.001);
    }

    #[test]
    fn cast_emits_aoe_stun() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();

        let spell = TailSweepSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_aoes.len(), 1);
        assert_eq!(ctx.pending_visuals.len(), 1);
    }

    #[test]
    fn is_behind_caster_filters_correctly() {
        let forward = Vec3::Z;

        // Target directly behind
        let to_behind = Vec3::new(0.0, 0.0, -1.0);
        assert!(TailSweepSpell::is_behind_caster(forward, to_behind));

        // Target directly in front
        let to_front = Vec3::new(0.0, 0.0, 1.0);
        assert!(!TailSweepSpell::is_behind_caster(forward, to_front));

        // Target to the side (90°)
        let to_side = Vec3::new(1.0, 0.0, 0.0);
        assert!(!TailSweepSpell::is_behind_caster(forward, to_side));
    }
}
