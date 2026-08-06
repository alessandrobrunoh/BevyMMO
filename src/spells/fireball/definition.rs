//! Fireball spell implementation.
//!
//! Spell ranged ad area: colpisce i target vicini al punto scelto dal client.
//! La simulazione resta server-authoritative: il client invia solo un comando
//! intenzionale (`SpellCastCommand`), mentre il server valida spellbook/cooldown
//! ed emette `DamageEvent`.

use bevy::math::Vec3;

use crate::plugins::spells::{Spell, SpellCastContext, SpellConfig, SpellId};

/// Fireball: projectile-like ranged AoE spell.
pub struct FireballSpell;

impl FireballSpell {
    pub const ID: &'static str = "fireball";
    pub const DISPLAY_NAME: &'static str = "Fireball";
    pub const COOLDOWN_SECONDS: f32 = 1.2;
    pub const CAST_RANGE: f32 = 8.0;
    pub const AREA_RADIUS: f32 = 1.5;
    pub const DAMAGE_MULTIPLIER: f32 = 1.4;

    pub fn fallback_target(caster_position: Vec3) -> Vec3 {
        caster_position + Vec3::Z * Self::CAST_RANGE
    }

    pub fn clamp_target_to_range(caster_position: Vec3, target_position: Vec3) -> Vec3 {
        let offset = target_position - caster_position;
        let distance = offset.length();
        if distance <= Self::CAST_RANGE || distance <= f32::EPSILON {
            target_position
        } else {
            caster_position + offset / distance * Self::CAST_RANGE
        }
    }

    pub fn is_in_blast_radius(center: Vec3, target: Vec3, radius: f32) -> bool {
        center.distance_squared(target) <= radius.max(0.0).powi(2)
    }
}

impl Spell for FireballSpell {
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
        let requested_center = ctx
            .target_position
            .unwrap_or_else(|| Self::fallback_target(ctx.caster_position));
        let center = Self::clamp_target_to_range(ctx.caster_position, requested_center);
        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        // Broadcast visual effect to all clients via server
        ctx.emit_visual(Self::ID, ctx.caster_position, center);

        for (target, position) in ctx.potential_targets.iter().copied() {
            if target == ctx.caster {
                continue;
            }
            if Self::is_in_blast_radius(center, position, Self::AREA_RADIUS) {
                ctx.emit_damage(target, damage);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blast_radius_includes_boundary_and_excludes_outside() {
        let center = Vec3::ZERO;
        assert!(FireballSpell::is_in_blast_radius(
            center,
            Vec3::new(1.5, 0.0, 0.0),
            1.5
        ));
        assert!(!FireballSpell::is_in_blast_radius(
            center,
            Vec3::new(1.51, 0.0, 0.0),
            1.5
        ));
    }

    #[test]
    fn target_is_clamped_to_cast_range() {
        let caster = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, 20.0);
        let clamped = FireballSpell::clamp_target_to_range(caster, target);
        assert_eq!(clamped, Vec3::new(0.0, 0.0, FireballSpell::CAST_RANGE));
    }

    #[test]
    fn target_inside_range_is_preserved() {
        let caster = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, 4.0);
        assert_eq!(FireballSpell::clamp_target_to_range(caster, target), target);
    }
}
