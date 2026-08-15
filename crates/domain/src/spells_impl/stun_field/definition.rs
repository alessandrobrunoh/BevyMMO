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

    /// Clamps the target point to `CAST_RANGE` around the caster.
    fn clamp_target_to_range(caster_position: Vec3, target: Vec3) -> Vec3 {
        let offset = target - caster_position;
        let horizontal = Vec3::new(offset.x, 0.0, offset.z);
        let distance = horizontal.length();
        if distance <= Self::CAST_RANGE {
            Vec3::new(target.x, 0.0, target.z)
        } else {
            let direction = horizontal / distance;
            caster_position + direction * Self::CAST_RANGE
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
