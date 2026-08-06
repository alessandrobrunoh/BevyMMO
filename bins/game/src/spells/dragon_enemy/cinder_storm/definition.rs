//! Cinder Storm spell implementation.
//!
//! 2.0s CastTime AoE that places 2 delayed fire circles at the densest
//! cluster of 2 players. After 1.5s delay, both circles impact with heavy damage.

use bevy::prelude::{Entity, Vec3};

use crate::plugins::entity::boss::target_select::{densest_cluster, PlayerRef};
use crate::plugins::spells::{
    AoeEffect, AoeTargeting, Spell, SpellCastContext, SpellConfig, SpellId,
};

pub struct CinderStormSpell;

impl CinderStormSpell {
    pub const ID: &'static str = "cinder_storm";
    pub const DISPLAY_NAME: &'static str = "Cinder Storm";
    pub const COOLDOWN_SECONDS: f32 = 12.0;
    pub const CAST_RANGE: f32 = 16.0;
    pub const AREA_RADIUS: f32 = 3.0;
    pub const CAST_TIME_SECONDS: f32 = 2.0;
    pub const IMPACT_DELAY_SECONDS: f32 = 1.5;
    pub const DAMAGE_MULTIPLIER: f32 = 2.5;
    pub const SECOND_CIRCLE_OFFSET: f32 = 2.0;

    /// Converts potential_targets to PlayerRef slice for target selection.
    fn to_player_refs<'a>(
        potential_targets: &[(Entity, Vec3)],
        buffer: &'a mut Vec<PlayerRef>,
    ) -> &'a [PlayerRef] {
        buffer.clear();
        buffer.extend(potential_targets.iter().map(|(entity, pos)| PlayerRef {
            entity: *entity,
            position: *pos,
        }));
        buffer
    }
}

impl Spell for CinderStormSpell {
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
        let mut player_buffer = Vec::new();
        let players = Self::to_player_refs(ctx.potential_targets, &mut player_buffer);

        let centroid = if let Some(center) = densest_cluster(players, 2) {
            center
        } else {
            // Fallback to caster position if not enough players
            ctx.caster_position
        };

        // Create two offset centers
        let center1 = centroid;
        let center2 = centroid + Vec3::X * Self::SECOND_CIRCLE_OFFSET;

        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        // Emit first delayed circle
        ctx.emit_aoe_with_delay(
            center1,
            Self::AREA_RADIUS,
            Self::IMPACT_DELAY_SECONDS,
            Self::IMPACT_DELAY_SECONDS,
            Self::ID,
            AoeEffect::Damage {
                amount: damage,
                targeting: AoeTargeting::ExcludeCaster,
            },
        );

        // Emit second delayed circle
        ctx.emit_aoe_with_delay(
            center2,
            Self::AREA_RADIUS,
            Self::IMPACT_DELAY_SECONDS,
            Self::IMPACT_DELAY_SECONDS,
            Self::ID,
            AoeEffect::Damage {
                amount: damage,
                targeting: AoeTargeting::ExcludeCaster,
            },
        );

        // Visual at centroid (shows both circles)
        ctx.emit_visual(Self::ID, centroid, centroid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::spells::context::SpellCastContext;
    use crate::stats::components::CombatStats;
    use bevy::prelude::World;

    #[test]
    fn cinder_storm_has_expected_id_and_config() {
        let spell = CinderStormSpell;
        assert_eq!(spell.id().as_str(), "cinder_storm");
        assert_eq!(spell.display_name(), "Cinder Storm");
        assert_eq!(spell.config().cooldown_seconds, 12.0);
        assert_eq!(spell.config().cast_time_seconds, 2.0);
    }

    #[test]
    fn cast_emits_two_delayed_aoe_circles() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let player1 = world.spawn_empty().id();
        let player2 = world.spawn_empty().id();

        let spell = CinderStormSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        // Two players clustered at (2,0) and (3,0) - centroid should be (2.5, 0)
        let potential = vec![
            (player1, Vec3::new(2.0, 0.0, 0.0)),
            (player2, Vec3::new(3.0, 0.0, 0.0)),
        ];

        let mut ctx =
            SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &potential);

        spell.cast(&mut ctx);

        assert_eq!(ctx.pending_aoes.len(), 2);
        assert_eq!(ctx.pending_visuals.len(), 1);

        // Both should be delayed damage effects
        for aoe in &ctx.pending_aoes {
            if let AoeEffect::Damage { amount, .. } = &aoe.effect {
                assert_eq!(*amount, 125.0); // 50 * 2.5
            } else {
                panic!("Expected Damage effect");
            }
        }

        // Centers should be offset by 2.0 on X axis
        let center1 = ctx.pending_aoes[0].center;
        let center2 = ctx.pending_aoes[1].center;
        assert!((center1.x - center2.x).abs() - 2.0 < 0.01);
    }

    #[test]
    fn cast_falls_back_to_caster_position_without_enough_players() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let player1 = world.spawn_empty().id();

        let spell = CinderStormSpell;
        let combat = CombatStats {
            attack_power: 50.0,
            armor: 0.0,
        };

        // Only one player - not enough for densest_cluster
        let potential = vec![(player1, Vec3::new(5.0, 0.0, 0.0))];

        let mut ctx = SpellCastContext::new(
            caster,
            Vec3::new(10.0, 0.0, 0.0),
            &combat,
            Vec3::Z,
            None,
            None,
            &potential,
        );

        spell.cast(&mut ctx);

        // Should still emit 2 circles at caster position
        assert_eq!(ctx.pending_aoes.len(), 2);

        // Both centers should be at caster position
        let caster_pos = Vec3::new(10.0, 0.0, 0.0);
        assert!((ctx.pending_aoes[0].center - caster_pos).length() < 0.01);
        assert!((ctx.pending_aoes[1].center - (caster_pos + Vec3::X * 2.0)).length() < 0.01);
    }
}
