//! Healing Circle spell implementation.
//!
//! Spawns a healing circle on the ground that applies a HoT to entities
//! that walk into it.

use crate::spells::{
    AoeEffect, AoeTargeting, Spell, SpellCastContext, SpellConfig, SpellId,
};
use crate::stats::events::{ModifierEffect, ModifierKind};

pub struct HealingCircleSpell;

impl HealingCircleSpell {
    pub const ID: &'static str = "healing_circle";
    pub const DISPLAY_NAME: &'static str = "Healing Circle";
    pub const COOLDOWN_SECONDS: f32 = 20.0;
    pub const CAST_RANGE: f32 = 12.0;
    pub const AREA_RADIUS: f32 = 4.0;

    /// HP healed for each tick of the HoT applied to anyone entering the circle.
    pub const HOT_AMOUNT_PER_TICK: f32 = 10.0;
    /// Interval (in seconds) between consecutive heal ticks.
    pub const HOT_TICK_INTERVAL: f32 = 0.5;
    /// Duration of the HoT applied to a target that entered the circle.
    /// Independent of `area_radius`: someone entering at the last second still
    /// receives a full duration HoT.
    pub const HOT_DURATION_SECONDS: f32 = 3.0;
}

impl Spell for HealingCircleSpell {
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
        let center = ctx.effective_center();

        // The HoT effect is now a payload carried by the spell: the central
        // `update_aoe_regions` system knows nothing about "healing_circle".
        // `AoeTargeting::CasterOnly` guarantees that the heal is applied
        // exclusively to the caster who generated the circle.
        let effect = AoeEffect::ApplyModifier {
            effects: vec![ModifierEffect::HealOverTime {
                amount_per_tick: Self::HOT_AMOUNT_PER_TICK,
                tick_interval: Self::HOT_TICK_INTERVAL,
            }],
            duration_seconds: Some(Self::HOT_DURATION_SECONDS),
            kind: ModifierKind::Buff,
            once_per_entity: true,
            targeting: AoeTargeting::CasterOnly,
        };

        ctx.emit_aoe(center, Self::AREA_RADIUS, 3.0, Self::ID, effect);

        ctx.emit_visual(Self::ID, center, center);
    }
}
