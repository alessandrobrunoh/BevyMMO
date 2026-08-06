//! Fireball: spell that spawns a homing projectile toward the selected target.
//!
//! Unlike RayOfLight (position-based AoE), Fireball creates a
//! projectile entity that pursues the target entity. Damage is applied
//! on contact by the `update_homing_projectiles` system.

use crate::plugins::spells::{Spell, SpellCastContext, SpellConfig, SpellId, TargetingMode};

/// Fireball: homing projectile spell.
pub struct FireballSpell;

impl FireballSpell {
    pub const ID: &'static str = "fireball";
    pub const DISPLAY_NAME: &'static str = "Fireball";
    pub const COOLDOWN_SECONDS: f32 = 10.0;
    /// Projectile speed in units/second.
    ///
    /// Must be significantly higher than player movement speed
    /// (which in `FixedUpdate` travels at ~9 units/sec base,
    /// plus any modifiers like `Swift`), otherwise the target can
    /// simply run away and outrun the projectile.
    pub const PROJECTILE_SPEED: f32 = 24.0;
    pub const HIT_RADIUS: f32 = 0.5;
    pub const DAMAGE_MULTIPLIER: f32 = 1.2;
}

impl Spell for FireballSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::ranged_single_target(Self::COOLDOWN_SECONDS, 15.0, TargetingMode::SingleEntity)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let Some(target) = ctx.target_entity else {
            // No target entity: do nothing
            return;
        };

        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        ctx.emit_projectile(target, Self::PROJECTILE_SPEED, damage, Self::HIT_RADIUS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fireball_has_expected_id() {
        assert_eq!(FireballSpell::ID, "fireball");
    }
}
