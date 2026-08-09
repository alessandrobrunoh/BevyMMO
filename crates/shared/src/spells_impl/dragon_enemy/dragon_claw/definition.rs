//! Definition of the `dragon_claw` spell.


use crate::spells::{Spell, SpellCastContext, SpellConfig, SpellId};

/// Instant melee strike on the current target.
///
/// Damage scales with the caster's `attack_power`. The boss rotation always
/// supplies `target_entity`, so `cast` no-ops safely when it is missing.
pub struct DragonClawSpell;

impl DragonClawSpell {
    pub const ID: &'static str = "dragon_claw";
    pub const DISPLAY_NAME: &'static str = "Dragon Claw";
    pub const COOLDOWN_SECONDS: f32 = 1.5;
    pub const DAMAGE_MULTIPLIER: f32 = 1.0;
}

impl Spell for DragonClawSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        // Melee profile: short cooldown, melee area radius (the spell itself is
        // single-target via `target_entity`, but `melee_aoe` gives the right
        // cast-time/range defaults for an instant melee hit).
        SpellConfig::melee_aoe(Self::COOLDOWN_SECONDS, 2.5)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let Some(target) = ctx.target_entity else {
            return;
        };
        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;
        ctx.emit_damage(target, damage);

        // Visual: anchor the slash at the target's position so the client draws
        // the arc where the hit lands.
        let target_position = ctx
            .potential_targets
            .iter()
            .find(|(entity, _)| *entity == target)
            .map(|(_, position)| *position)
            .unwrap_or(ctx.caster_position);
        ctx.emit_visual(Self::ID, target_position, target_position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spells::context::SpellCastContext;
    use crate::stats::components::CombatStats;
    use bevy::math::Vec3;
    use bevy::prelude::World;

    #[test]
    fn dragon_claw_has_expected_id_and_config() {
        let spell = DragonClawSpell;
        assert_eq!(spell.id().as_str(), "dragon_claw");
        assert_eq!(spell.display_name(), "Dragon Claw");
        assert_eq!(spell.config().cooldown_seconds, 1.5);
    }

    #[test]
    fn cast_emits_damage_when_a_target_is_supplied() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let target = world.spawn_empty().id();

        let spell = DragonClawSpell;
        let combat = CombatStats {
            attack_power: 30.0,
            armor: 0.0,
        };
        let potential = vec![(target, Vec3::new(1.0, 0.0, 0.0))];

        let mut ctx = SpellCastContext::new(
            caster,
            Vec3::ZERO,
            &combat,
            Vec3::Z,
            None,
            Some(target),
            &potential,
        );

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_damage.len(), 1);
        assert_eq!(ctx.pending_damage[0].target, target);
        assert_eq!(ctx.pending_damage[0].amount, 30.0);
        assert_eq!(ctx.pending_visuals.len(), 1);
    }

    #[test]
    fn cast_is_a_noop_without_a_target_entity() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();

        let spell = DragonClawSpell;
        let combat = CombatStats {
            attack_power: 30.0,
            armor: 0.0,
        };

        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        spell.cast(&mut ctx);

        assert!(ctx.pending_damage.is_empty());
        assert!(ctx.pending_visuals.is_empty());
    }
}
