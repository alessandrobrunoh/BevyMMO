//! Followball: spell che spawna un proiettile homing verso il target selezionato.
//!
//! A differenza della Fireball (AoE su posizione), la Followball crea una
//! entity projectile che insegue il target entity. Il danno viene applicato
//! al contatto dal sistema `update_homing_projectiles`.

use crate::plugins::spells::{Spell, SpellCastContext, SpellConfig, SpellId, TargetingMode};

/// Followball: homing projectile spell.
pub struct FollowballSpell;

impl FollowballSpell {
    pub const ID: &'static str = "followball";
    pub const DISPLAY_NAME: &'static str = "Followball";
    pub const COOLDOWN_SECONDS: f32 = 0.8;
    pub const PROJECTILE_SPEED: f32 = 8.0;
    pub const HIT_RADIUS: f32 = 0.5;
    pub const DAMAGE_MULTIPLIER: f32 = 1.2;
}

impl Spell for FollowballSpell {
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
            // Nessun target entity: non fare nulla
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
    fn followball_has_expected_id() {
        assert_eq!(FollowballSpell::ID, "followball");
    }
}
