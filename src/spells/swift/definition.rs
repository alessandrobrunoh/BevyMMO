//! Swift spell implementation.
//!
//! Channeling self-buff: mentre il caster tiene premuto F riceve un modifier
//! `Speed * 1.20` che viene refreshato ogni `TICK_INTERVAL_SECONDS`. Il
//! channel dura al massimo `CHANNEL_DURATION_SECONDS`; rilasciare F lo interrompe
//! prima. Il modifier scade naturalmente poco dopo l'ultimo tick.

use crate::plugins::spells::{
    ChannelMovementPolicy, ModifierEffect, ModifierKind, Spell, SpellCastContext, SpellConfig,
    SpellId, TargetingMode,
};
use crate::stats::events::{ModifierOp, StatField};

pub struct SwiftSpell;

impl SwiftSpell {
    pub const ID: &'static str = "swift";
    pub const DISPLAY_NAME: &'static str = "Swift";
    pub const COOLDOWN_SECONDS: f32 = 10.0;
    pub const SPEED_MULTIPLIER: f32 = 1.35;
    pub const TICK_INTERVAL_SECONDS: f32 = 0.25;
    pub const CHANNEL_DURATION_SECONDS: f32 = 4.0;
    pub const MODIFIER_DURATION_SECONDS: f32 = 0.5;
}

impl Spell for SwiftSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::new(
            Self::COOLDOWN_SECONDS,
            0.0,
            0.0,
            TargetingMode::SelfCentered,
        )
        .with_channel(ChannelMovementPolicy::AllowMovement)
        .with_channel_duration(Self::CHANNEL_DURATION_SECONDS)
    }

    fn channel_tick_interval_seconds(&self) -> f32 {
        Self::TICK_INTERVAL_SECONDS
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        ctx.emit_modifier(
            ctx.caster,
            vec![ModifierEffect::Stat {
                field: StatField::Speed,
                operation: ModifierOp::Multiply,
                value: Self::SPEED_MULTIPLIER,
            }],
            Some(Self::MODIFIER_DURATION_SECONDS),
            ModifierKind::Buff,
        );
    }
}
