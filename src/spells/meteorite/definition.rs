//! Meteorite spell implementation.
//!
//! 1s CastTime → al completamento appare un cerchio di warning sul punto target;
//! dopo 2 secondi (`IMPACT_DELAY_SECONDS`) il meteorite impatta: 50 danni a
//! tutte le entità nel raggio, tranne il caster (`ExcludeCaster`).

use bevy::prelude::Vec3;

use crate::plugins::spells::{
    AoeEffect, AoeTargeting, Spell, SpellCastContext, SpellConfig, SpellId,
};

pub struct MeteoriteSpell;

impl MeteoriteSpell {
    pub const ID: &'static str = "meteorite";
    pub const DISPLAY_NAME: &'static str = "Meteorite";
    pub const COOLDOWN_SECONDS: f32 = 8.0;
    pub const CAST_RANGE: f32 = 14.0;
    pub const AREA_RADIUS: f32 = 3.5;
    pub const CAST_TIME_SECONDS: f32 = 1.0;
    pub const IMPACT_DELAY_SECONDS: f32 = 2.0;
    pub const DAMAGE: f32 = 50.0;

    /// Clampa il punto target al `CAST_RANGE` attorno al caster.
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

impl Spell for MeteoriteSpell {
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
        let center = ctx
            .target_position
            .map(|target| Self::clamp_target_to_range(ctx.caster_position, target))
            .unwrap_or(ctx.caster_position);

        // Payload Damage burst con delay iniziale di IMPACT_DELAY_SECONDS.
        // La regione vive esattamente il tempo del delay: al tick di impatto
        // applica il danno una tantum e despawna.
        let effect = AoeEffect::Damage {
            amount: Self::DAMAGE,
            targeting: AoeTargeting::ExcludeCaster,
        };

        ctx.emit_aoe_with_delay(
            center,
            Self::AREA_RADIUS,
            Self::IMPACT_DELAY_SECONDS,
            Self::IMPACT_DELAY_SECONDS,
            Self::ID,
            effect,
        );

        ctx.emit_visual(Self::ID, center, center);
    }
}
