//! Stun Field spell implementation.
//!
//! Instant-cast AoE spell that creates a stun field after a short delay.
//! The caster places a warning circle on the ground; after 0.5 seconds
//! (`IMPACT_DELAY_SECONDS`) the field activates and stuns all enemies
//! in the area for 2 seconds, excluding the caster.

use glam::Vec3;

use crate::crowd_control::CrowdControlKind;
use crate::spells::{AoeEffect, AoeTargeting, Spell, SpellCastContext, SpellConfig, SpellId};

pub struct StunFieldSpell;

impl StunFieldSpell {
    pub const ID: &'static str = "stun_field";
    pub const DISPLAY_NAME: &'static str = "Stun Field";
    pub const COOLDOWN_SECONDS: f32 = 15.0;
    pub const CAST_RANGE: f32 = 12.0;
    pub const AREA_RADIUS: f32 = 4.0;
    pub const IMPACT_DELAY_SECONDS: f32 = 0.5;
    pub const STUN_DURATION_SECONDS: f32 = 2.0;

    /// Clamps the target point to `CAST_RANGE` around the caster, preserving height.
    fn clamp_target_to_range(caster_position: Vec3, target: Vec3) -> Vec3 {
        let offset = target - caster_position;
        let horizontal = Vec3::new(offset.x, 0.0, offset.z);
        let distance = horizontal.length();
        if distance <= Self::CAST_RANGE {
            target
        } else {
            let direction = horizontal / distance;
            Vec3::new(
                caster_position.x + direction.x * Self::CAST_RANGE,
                caster_position.y + (target.y - caster_position.y) * (Self::CAST_RANGE / distance),
                caster_position.z + direction.z * Self::CAST_RANGE,
            )
        }
    }
}

impl Spell for StunFieldSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::ranged_aoe(Self::COOLDOWN_SECONDS, Self::CAST_RANGE, Self::AREA_RADIUS)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let center = ctx
            .target_position
            .map(|target| Self::clamp_target_to_range(ctx.caster_position, target))
            .unwrap_or(ctx.caster_position);

        ctx.emit_aoe_with_delay(
            center,
            Self::AREA_RADIUS,
            Self::IMPACT_DELAY_SECONDS,
            Self::IMPACT_DELAY_SECONDS,
            Self::ID,
            AoeEffect::CrowdControl {
                kind: CrowdControlKind::Stun,
                duration_seconds: Self::STUN_DURATION_SECONDS,
                once_per_entity: true,
                targeting: AoeTargeting::ExcludeCaster,
            },
        );

        ctx.emit_visual(Self::ID, center, center);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stun_field_clamp_preserves_target_elevation() {
        let caster = Vec3::new(0.0, 5.0, 0.0);
        let target_on_mountain = Vec3::new(3.0, 16.0, 4.0);
        let clamped = StunFieldSpell::clamp_target_to_range(caster, target_on_mountain);
        assert_eq!(clamped, target_on_mountain);
        assert_eq!(clamped.y, 16.0);
    }

    #[test]
    fn stun_field_clamp_interpolates_elevation_when_out_of_range() {
        let caster = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(0.0, 24.0, 24.0); // distance = 24.0, range = 12.0 (halfway)
        let clamped = StunFieldSpell::clamp_target_to_range(caster, target);
        assert!((clamped.y - 12.0).abs() < 1e-4);
        assert!((clamped.z - 12.0).abs() < 1e-4);
    }
}

