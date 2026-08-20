//! Dragon rotation spells. Instant payloads so AI `request_cast` can fire them.

use bevymmo_gameplay::effects::{DamageEffect, EffectBundle, EffectContext, EffectSpec};
use bevymmo_gameplay::spells::context::AoeShape;
use bevymmo_props_macro::spell;

use crate::spells::{SpellCast, SpellCastContext, SpellRegistry};
use crate::EntityId;

fn boss_damage(ctx: &SpellCastContext, multiplier: f32) -> EffectSpec {
    EffectSpec::Damage(DamageEffect {
        amount: ctx.caster_combat.attack_power * multiplier,
    })
}

fn apply_damage(ctx: &mut SpellCastContext, target: EntityId, multiplier: f32) {
    let mut context = EffectContext::new(target);
    context.source = Some(ctx.caster);
    let effect = boss_damage(ctx, multiplier);
    ctx.pending_effects
        .push(EffectBundle::single(context, effect));
}

#[spell(id = "dragon_claw", name = "Dragon Claw", cooldown = 4.0, targeting = SingleEntity, range = 4.0)]
pub struct DragonClaw;

impl SpellCast for DragonClaw {
    fn cast(&self, ctx: &mut SpellCastContext) {
        let Some(target) = ctx.target_entity else {
            return;
        };
        apply_damage(ctx, target, 1.4);
    }
}

#[spell(
    id = "searing_breath",
    name = "Searing Breath",
    cooldown = 8.0,
    targeting = SingleEntity,
    range = 12.0
)]
pub struct SearingBreath;

impl SpellCast for SearingBreath {
    fn cast(&self, ctx: &mut SpellCastContext) {
        let center = ctx.caster_position;
        ctx.emit_aoe_excluding_caster(
            center,
            10.0,
            AoeShape::Cone {
                direction: ctx.caster_look_direction,
                angle_deg: 50.0,
            },
            0.0,
            0.0,
            Self::ID,
            vec![boss_damage(ctx, 1.6)],
        );
    }
}

#[spell(id = "cinder_storm", name = "Cinder Storm", cooldown = 12.0, targeting = GroundAoe, range = 16.0, area = 5.0)]
pub struct CinderStorm;

impl SpellCast for CinderStorm {
    fn cast(&self, ctx: &mut SpellCastContext) {
        let center = ctx.target_position.unwrap_or(ctx.caster_position);
        ctx.emit_aoe_excluding_caster(
            center,
            5.0,
            AoeShape::Circle,
            0.0,
            0.0,
            Self::ID,
            vec![boss_damage(ctx, 1.3)],
        );
    }
}

#[spell(id = "wing_buffet", name = "Wing Buffet", cooldown = 10.0, targeting = SelfCentered, area = 6.0)]
pub struct WingBuffet;

impl SpellCast for WingBuffet {
    fn cast(&self, ctx: &mut SpellCastContext) {
        ctx.emit_aoe_excluding_caster(
            ctx.caster_position,
            6.0,
            AoeShape::Circle,
            0.0,
            0.0,
            Self::ID,
            vec![boss_damage(ctx, 1.1)],
        );
    }
}

#[spell(id = "tail_sweep", name = "Tail Sweep", cooldown = 7.0, targeting = SelfCentered, area = 5.0)]
pub struct TailSweep;

impl SpellCast for TailSweep {
    fn cast(&self, ctx: &mut SpellCastContext) {
        ctx.emit_aoe_excluding_caster(
            ctx.caster_position,
            5.0,
            AoeShape::Circle,
            0.0,
            0.0,
            Self::ID,
            vec![boss_damage(ctx, 1.2)],
        );
    }
}

#[spell(id = "molten_eruption", name = "Molten Eruption", cooldown = 14.0, targeting = SelfCentered, area = 8.0)]
pub struct MoltenEruption;

impl SpellCast for MoltenEruption {
    fn cast(&self, ctx: &mut SpellCastContext) {
        ctx.emit_aoe_excluding_caster(
            ctx.caster_position,
            8.0,
            AoeShape::Circle,
            0.0,
            0.0,
            Self::ID,
            vec![boss_damage(ctx, 1.8)],
        );
    }
}

/// Boss-only spell. Distinct from the hammer `cataclysm` BaseAbility: AI
/// looks this up in `SpellRegistry`, players resolve abilities separately.
#[spell(id = "cataclysm", name = "Cataclysm", cooldown = 20.0, targeting = SelfCentered, area = 10.0)]
pub struct BossCataclysm;

impl SpellCast for BossCataclysm {
    fn cast(&self, ctx: &mut SpellCastContext) {
        ctx.emit_aoe_excluding_caster(
            ctx.caster_position,
            10.0,
            AoeShape::Circle,
            0.0,
            0.0,
            Self::ID,
            vec![boss_damage(ctx, 2.2)],
        );
    }
}

pub fn register(registry: &mut SpellRegistry) {
    DragonClaw::register(registry);
    SearingBreath::register(registry);
    CinderStorm::register(registry);
    WingBuffet::register(registry);
    TailSweep::register(registry);
    MoltenEruption::register(registry);
    BossCataclysm::register(registry);
}

/// Every id the boss rotation tables mention.
pub const BOSS_ROTATION_SPELL_IDS: &[&str] = &[
    "searing_breath",
    "cinder_storm",
    "wing_buffet",
    "tail_sweep",
    "dragon_claw",
    "molten_eruption",
    "cataclysm",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_definitions::default_spells;
    use crate::spells::registry::SpellId;

    #[test]
    fn default_spells_contains_every_boss_rotation_id() {
        let registry = default_spells();
        assert!(registry.contains(&SpellId::new("fireball")));
        for id in BOSS_ROTATION_SPELL_IDS {
            assert!(
                registry.contains(&SpellId::new(*id)),
                "boss rotation id {id} must be registered"
            );
        }
    }

    #[test]
    fn boss_cataclysm_shares_the_rotation_string() {
        assert_eq!(BossCataclysm::ID, "cataclysm");
    }
}
