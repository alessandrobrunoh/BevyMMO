//! Shared pure helpers for point-and-click movement presentation.

use crate::spells::{CastKind, CastProgress};
use crate::stats::events::ModifierOp;
use crate::stats::events::StatField;
use crate::stats::modifiers::ActiveStatModifiers;
use crate::stats::modifiers::StatModifierInstance;

/// Calculates movement speed after active stat modifiers.
///
/// This is shared by gameplay and the stats UI so the value displayed to the
/// player matches the speed used by gameplay.
pub fn effective_movement_speed(base_speed: f32, modifiers: Option<&ActiveStatModifiers>) -> f32 {
    let Some(active) = modifiers else {
        return base_speed;
    };
    effective_value(StatField::Speed, base_speed, &active.modifiers)
}

fn effective_value(field: StatField, base: f32, modifiers: &[StatModifierInstance]) -> f32 {
    let mut result = base;
    let mut override_value: Option<f32> = None;

    for modifier in modifiers {
        for effect in &modifier.effects {
            if let crate::stats::modifiers::ModifierEffectInstance::Stat {
                field: effect_field,
                operation,
                value,
            } = effect
            {
                if *effect_field != field {
                    continue;
                }
                match operation {
                    ModifierOp::Add => result += value,
                    ModifierOp::Multiply => result *= value,
                    ModifierOp::Override => override_value = Some(*value),
                }
            }
        }
    }

    override_value.unwrap_or(result)
}

/// Returns true when a cast state must freeze point-and-click movement.
pub fn should_block_movement_for_cast(cast: Option<&CastProgress>) -> bool {
    let Some(cast) = cast else {
        return false;
    };
    match cast.kind {
        CastKind::CastTime => true,
        CastKind::Channeling => {
            cast.channel_movement == crate::spells::ChannelMovementPolicy::InterruptOnMove
        }
        CastKind::Instant => false,
    }
}
