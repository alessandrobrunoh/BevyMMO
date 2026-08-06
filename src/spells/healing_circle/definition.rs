//! Healing Circle spell implementation.
//!
//! Spawns a healing circle on the ground that applies a HoT to entities
//! that walk into it.

use crate::plugins::spells::{AoeEffect, Spell, SpellCastContext, SpellConfig, SpellId};
use crate::stats::events::{ModifierEffect, ModifierKind};

pub struct HealingCircleSpell;

impl HealingCircleSpell {
    pub const ID: &'static str = "healing_circle";
    pub const DISPLAY_NAME: &'static str = "Healing Circle";
    pub const COOLDOWN_SECONDS: f32 = 20.0;
    pub const CAST_RANGE: f32 = 12.0;
    pub const AREA_RADIUS: f32 = 4.0;

    /// HP curati per ogni tick dell'HoT applicato a chi entra nel cerchio.
    pub const HOT_AMOUNT_PER_TICK: f32 = 10.0;
    /// Intervallo (in secondi) tra un tick di cura e il successivo.
    pub const HOT_TICK_INTERVAL: f32 = 0.5;
    /// Durata dell'HoT applicato a un bersaglio entrato nel cerchio.
    /// Indipendente da `area_radius`: chi entra all'ultimo secondo riceve
    /// comunque un HoT a durata piena.
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

        // L'effetto dell'HoT è ora un payload portato dalla spell: il sistema
        // centrale `update_aoe_regions` non sa più nulla di "healing_circle".
        // `AoeTargeting::CasterOnly` garantisce che la cura venga applicata
        // esclusivamente al caster che ha generato il cerchio.
        let effect = AoeEffect::ApplyModifier {
            effects: vec![ModifierEffect::HealOverTime {
                amount_per_tick: Self::HOT_AMOUNT_PER_TICK,
                tick_interval: Self::HOT_TICK_INTERVAL,
            }],
            duration_seconds: Some(Self::HOT_DURATION_SECONDS),
            kind: ModifierKind::Buff,
            once_per_entity: true,
            targeting: crate::plugins::spells::AoeTargeting::CasterOnly,
        };

        ctx.emit_aoe(center, Self::AREA_RADIUS, 3.0, Self::ID, effect);

        ctx.emit_visual(Self::ID, center, center);
    }
}
