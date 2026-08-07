//! The basic Attack spell implementation.

use bevy::math::Vec3;

use crate::spells::{Spell, SpellCastContext, SpellConfig, SpellId};

/// A basic melee attack spell that damages all enemies in range.
///
/// This is a simple area-of-effect attack centered on the caster. It's
/// designed as a template for more complex spells and as the default
/// attack for new characters.
pub struct AttackSpell;

impl AttackSpell {
    /// The unique identifier for this spell.
    pub const ID: &'static str = "attack";

    /// The human-readable display name.
    pub const DISPLAY_NAME: &'static str = "Attack";

    /// Cooldown time in seconds.
    pub const COOLDOWN_SECONDS: f32 = 1.0;

    /// Area of effect radius in world units.
    pub const AREA_RADIUS: f32 = 3.0;

    /// Cast range (0.0 = centered on caster).
    pub const CAST_RANGE: f32 = 0.0;

    /// Check if a target position is within the attack radius of a center point.
    ///
    /// This is a pure helper function that can be easily tested.
    pub fn is_in_attack_radius(center: Vec3, target: Vec3, radius: f32) -> bool {
        center.distance(target) <= radius
    }
}

impl Spell for AttackSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::melee_aoe(Self::COOLDOWN_SECONDS, Self::AREA_RADIUS)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let center = ctx.caster_position;
        let radius = Self::AREA_RADIUS;

        // Filter targets by area radius
        let targets_in_range = ctx.targets_in_radius(center, radius);

        // Apply damage to each target in range (excluding self)
        let damage_amount = ctx.caster_combat.attack_power;

        for (target, _) in targets_in_range {
            if target != ctx.caster {
                ctx.emit_damage(target, damage_amount);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_attack_radius() {
        let center = Vec3::new(0.0, 0.0, 0.0);
        let radius = 3.0;

        // Target exactly at center
        assert!(AttackSpell::is_in_attack_radius(center, center, radius));

        // Target at edge of radius
        let at_edge = Vec3::new(3.0, 0.0, 0.0);
        assert!(AttackSpell::is_in_attack_radius(center, at_edge, radius));

        // Target just outside radius
        let outside = Vec3::new(3.1, 0.0, 0.0);
        assert!(!AttackSpell::is_in_attack_radius(center, outside, radius));

        // Target well outside radius
        let far_outside = Vec3::new(10.0, 0.0, 0.0);
        assert!(!AttackSpell::is_in_attack_radius(
            center,
            far_outside,
            radius
        ));
    }

    #[test]
    fn test_is_in_attack_radius_3d() {
        let center = Vec3::new(0.0, 0.0, 0.0);
        let radius = 5.0;

        // Target at same distance but in different directions
        let target1 = Vec3::new(3.0, 4.0, 0.0); // 3-4-5 triangle = 5.0 distance
        assert!(AttackSpell::is_in_attack_radius(center, target1, radius));

        let target2 = Vec3::new(3.0, 0.0, 4.0); // Also 5.0 distance
        assert!(AttackSpell::is_in_attack_radius(center, target2, radius));

        let target3 = Vec3::new(5.0, 0.0, 0.1); // Slightly over 5.0
        assert!(!AttackSpell::is_in_attack_radius(center, target3, radius));
    }

    #[test]
    fn test_is_in_attack_radius_zero_radius() {
        let center = Vec3::new(0.0, 0.0, 0.0);
        let radius = 0.0;

        // Only the exact center is within range
        assert!(AttackSpell::is_in_attack_radius(center, center, radius));

        let nearby = Vec3::new(0.0001, 0.0, 0.0);
        assert!(!AttackSpell::is_in_attack_radius(center, nearby, radius));
    }

    #[test]
    fn test_attack_spell_config() {
        let spell = AttackSpell;
        let config = spell.config();

        assert_eq!(config.cooldown_seconds, AttackSpell::COOLDOWN_SECONDS);
        assert_eq!(config.cast_range, AttackSpell::CAST_RANGE);
        assert_eq!(config.area_radius, AttackSpell::AREA_RADIUS);
    }

    #[test]
    fn test_attack_spell_id() {
        let spell = AttackSpell;
        let id = spell.id();

        assert_eq!(id.as_str(), AttackSpell::ID);
    }

    #[test]
    fn test_attack_spell_display_name() {
        let spell = AttackSpell;
        assert_eq!(spell.display_name(), AttackSpell::DISPLAY_NAME);
    }
}
