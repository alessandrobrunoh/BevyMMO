//! Searing Breath spell implementation.
//!
//! 1.5s CastTime front cone attack (range 14.0, half-angle cos 0.6).
//! Heavy damage to all targets within the cone in front of the caster.

use bevy::prelude::Vec3;

use crate::plugins::spells::{Spell, SpellCastContext, SpellConfig, SpellId};

pub struct SearingBreathSpell;

impl SearingBreathSpell {
    pub const ID: &'static str = "searing_breath";
    pub const DISPLAY_NAME: &'static str = "Searing Breath";
    pub const COOLDOWN_SECONDS: f32 = 8.0;
    pub const CAST_RANGE: f32 = 14.0;
    pub const AREA_RADIUS: f32 = 14.0;
    pub const CAST_TIME_SECONDS: f32 = 1.5;
    pub const CONE_COS_THRESHOLD: f32 = 0.6; // ~53° half-angle
    pub const DAMAGE_MULTIPLIER: f32 = 3.0;

    /// Filters targets within the front cone (angle AND range).
    fn is_in_cone(caster_look: Vec3, to_target: Vec3) -> bool {
        let forward_flat = Vec3::new(caster_look.x, 0.0, caster_look.z).normalize();
        let to_target_flat = Vec3::new(to_target.x, 0.0, to_target.z);
        let distance = to_target_flat.length();
        if distance > 1e-6 {
            let direction = to_target_flat / distance;
            forward_flat.dot(direction) >= Self::CONE_COS_THRESHOLD
        } else {
            true // Target at caster position counts as hit
        }
    }
}

impl Spell for SearingBreathSpell {
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
        let facing = ctx.caster_look_direction;

        for &(target, target_pos) in ctx.potential_targets {
            if target == ctx.caster {
                continue;
            }

            let to_target = target_pos - ctx.caster_position;
            let distance = to_target.length();

            if distance > Self::CAST_RANGE {
                continue;
            }

            if !Self::is_in_cone(facing, to_target) {
                continue;
            }

            ctx.emit_damage(target, damage);
        }

        // Visual anchored at caster, extending forward
        let visual_end = ctx.caster_position + facing.normalize() * Self::CAST_RANGE;
        ctx.emit_visual(Self::ID, ctx.caster_position, visual_end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::spells::context::SpellCastContext;
    use crate::stats::components::CombatStats;
    use bevy::prelude::World;

    #[test]
    fn searing_breath_has_expected_id_and_config() {
        let spell = SearingBreathSpell;
        assert_eq!(spell.id().as_str(), "searing_breath");
        assert_eq!(spell.display_name(), "Searing Breath");
        assert_eq!(spell.config().cooldown_seconds, 8.0);
        assert_eq!(spell.config().cast_time_seconds, 1.5);
    }

    #[test]
    fn cast_emits_damage_to_targets_in_cone() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let target_in_cone = world.spawn_empty().id();
        let target_behind = world.spawn_empty().id();

        let spell = SearingBreathSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        // Caster facing +Z, target at +5 (in cone), target at -5 (behind)
        let potential = vec![
            (target_in_cone, Vec3::new(0.0, 0.0, 5.0)),
            (target_behind, Vec3::new(0.0, 0.0, -5.0)),
        ];

        let mut ctx =
            SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &potential);

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_damage.len(), 1);
        assert_eq!(ctx.pending_damage[0].target, target_in_cone);
        assert_eq!(ctx.pending_damage[0].amount, 150.0); // 50 * 3.0
    }

    #[test]
    fn cast_filters_by_range() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let target_close = world.spawn_empty().id();
        let target_far = world.spawn_empty().id();

        let spell = SearingBreathSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        // Targets at 10 and 20 units (range is 14.0)
        let potential = vec![
            (target_close, Vec3::new(0.0, 0.0, 10.0)),
            (target_far, Vec3::new(0.0, 0.0, 20.0)),
        ];

        let mut ctx =
            SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &potential);

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_damage.len(), 1);
        assert_eq!(ctx.pending_damage[0].target, target_close);
    }

    #[test]
    fn is_in_cone_filters_correctly() {
        let forward = Vec3::Z;

        // Target directly in front
        let to_front = Vec3::new(0.0, 0.0, 1.0);
        assert!(SearingBreathSpell::is_in_cone(forward, to_front));

        // Target at 45° (cos 45° ≈ 0.707 > 0.6)
        let to_angle = Vec3::new(0.5, 0.0, 1.0).normalize();
        assert!(SearingBreathSpell::is_in_cone(forward, to_angle));

        // Target at edge (~53°, cos 53° ≈ 0.6)
        let to_edge = Vec3::new(0.8, 0.0, 1.0).normalize();
        assert!(SearingBreathSpell::is_in_cone(forward, to_edge));

        // Target outside cone (60°, cos 60° ≈ 0.5 < 0.6)
        let to_outside = Vec3::new(1.0, 0.0, 0.577).normalize(); // tan 60°
        assert!(!SearingBreathSpell::is_in_cone(forward, to_outside));
    }
}
