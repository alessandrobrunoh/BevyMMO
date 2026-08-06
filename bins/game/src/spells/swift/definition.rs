//! Swift spell implementation.
//!
//! Channeling self-buff: while the caster holds F they receive a
//! `Speed * 1.35` modifier that refreshes every `TICK_INTERVAL_SECONDS`. The
//! channel lasts at most `CHANNEL_DURATION_SECONDS`; releasing F interrupts it
//! earlier. The modifier expires naturally shortly after the last tick.

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
