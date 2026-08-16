//! Fireball: spell that spawns a homing projectile toward the selected target.
//!
//! Unlike RayOfLight (position-based AoE), Fireball creates a
//! projectile entity that pursues the target entity. Damage is applied
//! on contact by the `update_homing_projectiles` system.

use bevymmo_props_macro::spell;

use crate::spells::{SpellCast, SpellCastContext};
use crate::status_definitions::burn::Burn;

/// Projectile speed in units/second.
///
/// Must be significantly higher than player movement speed
/// (which in `FixedUpdate` travels at ~9 units/sec base,
/// plus any modifiers like `Swift`), otherwise the target can
/// simply run away and outrun the projectile.
const PROJECTILE_SPEED: f32 = 24.0;
const HIT_RADIUS: f32 = 0.5;
const DAMAGE_MULTIPLIER: f32 = 1.2;

#[spell(
    id = "fireball",
    name = "Fireball",
    cooldown = 10.0,
    targeting = SingleEntity,
    range = 15.0,
)]
pub struct FireballSpell;

impl SpellCast for FireballSpell {
    fn cast(&self, ctx: &mut SpellCastContext) {
        let Some(target) = ctx.target_entity else {
            // No target entity: do nothing
            return;
        };

        let damage = ctx.caster_combat.attack_power * DAMAGE_MULTIPLIER;

        ctx.emit_projectile_effects(
            target,
            PROJECTILE_SPEED,
            vec![
                bevymmo_gameplay::effects::EffectSpec::Damage(
                    bevymmo_gameplay::effects::DamageEffect { amount: damage },
                ),
                bevymmo_gameplay::effects::EffectSpec::ApplyStatus(
                    bevymmo_gameplay::effects::ApplyStatusEffect {
                        status_id: Burn::status_id(),
                        duration_override_seconds: None,
                        potency: 1.0,
                    },
                ),
            ],
            HIT_RADIUS,
        );
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
