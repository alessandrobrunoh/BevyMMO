//! Stats systems: applying damage/healing/modifiers, modifier expiration,
//! and death management.
//!
//! All systems that mutate gameplay state are server-authoritative.

use bevy::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

use bevymmo_shared::stats::components::{CombatStats, VitalStats};
use bevymmo_shared::stats::events::{
    ApplyStatModifierEvent, DamageEvent, HealEvent, ModifierEffect, ModifierKind, ModifierOp,
    StatField,
};
use bevymmo_shared::stats::formulas::damage_after_armor;
use bevymmo_shared::stats::modifiers::{
    ActiveStatModifiers, ModifierEffectInstance, ModifierId, StatModifierInstance,
};

static NEXT_MODIFIER_ID: AtomicU64 = AtomicU64::new(0);

/// Applies accumulated `DamageEvent`s: armor reduction + clamping.
pub fn apply_damage(
    mut events: MessageReader<DamageEvent>,
    mut targets: Query<(&mut VitalStats, &CombatStats)>,
) {
    for event in events.read() {
        let Ok((mut vital, combat)) = targets.get_mut(event.target) else {
            continue;
        };
        let effective = damage_after_armor(event.amount, &combat);
        vital.current_health = (vital.current_health - effective).max(0.0);
    }
}

/// Observer handler for `DamageEvent` triggers.
pub fn on_damage_triggered(
    trigger: On<DamageEvent>,
    mut targets: Query<(&mut VitalStats, &CombatStats)>,
) {
    let event = trigger.event();
    let Ok((mut vital, combat)) = targets.get_mut(event.target) else {
        return;
    };
    let effective = damage_after_armor(event.amount, combat);
    vital.current_health = (vital.current_health - effective).max(0.0);
}

/// Applies accumulated `HealEvent`s, clamping to maximum.
pub fn apply_healing(mut events: MessageReader<HealEvent>, mut targets: Query<&mut VitalStats>) {
    for event in events.read() {
        let Ok(mut vital) = targets.get_mut(event.target) else {
            continue;
        };
        vital.current_health = (vital.current_health + event.amount).min(vital.max_health);
    }
}

/// Observer handler for `HealEvent` triggers.
pub fn on_heal_triggered(
    trigger: On<HealEvent>,
    mut targets: Query<&mut VitalStats>,
) {
    let event = trigger.event();
    let Ok(mut vital) = targets.get_mut(event.target) else {
        return;
    };
    vital.current_health = (vital.current_health + event.amount).min(vital.max_health);
}

/// Converts `ApplyStatModifierEvent` into `StatModifierInstance` attached
/// to the target via `ActiveStatModifiers`.
pub fn apply_stat_modifiers(
    mut commands: Commands,
    mut events: MessageReader<ApplyStatModifierEvent>,
    mut targets: Query<&mut ActiveStatModifiers>,
) {
    for event in events.read() {
        let mut effect_instances = Vec::new();
        for effect in &event.effects {
            let instance = match effect {
                ModifierEffect::Stat {
                    field,
                    operation,
                    value,
                } => ModifierEffectInstance::Stat {
                    field: *field,
                    operation: *operation,
                    value: *value,
                },
                ModifierEffect::HealOverTime {
                    amount_per_tick,
                    tick_interval,
                } => ModifierEffectInstance::HealOverTime {
                    amount_per_tick: *amount_per_tick,
                    tick_interval: *tick_interval,
                    time_since_last_tick: 0.0,
                },
                ModifierEffect::DamageOverTime {
                    amount_per_tick,
                    tick_interval,
                } => ModifierEffectInstance::DamageOverTime {
                    amount_per_tick: *amount_per_tick,
                    tick_interval: *tick_interval,
                    time_since_last_tick: 0.0,
                },
            };
            effect_instances.push(instance);
        }

        let instance = StatModifierInstance {
            id: ModifierId(NEXT_MODIFIER_ID.fetch_add(1, Ordering::Relaxed)),
            source: event.source,
            effects: effect_instances,
            remaining_seconds: event.duration_seconds,
            kind: event.kind,
        };

        match targets.get_mut(event.target) {
            Ok(mut active) => refresh_or_insert_modifier(&mut active, instance),
            _ => {
                commands.entity(event.target).insert(ActiveStatModifiers {
                    modifiers: vec![instance],
                });
            }
        }
    }
}

/// Keeps repeating buffs stable without stacking identical timed effects.
///
/// Channeling spells refresh their modifier frequently. Reusing an existing
/// modifier avoids multiplicative speed explosions and reduces the number of
/// active modifiers that movement must scan every tick.
///
/// # Example
/// ```rust,ignore
/// refresh_or_insert_modifier(&mut active_modifiers, swift_modifier);
/// ```
fn refresh_or_insert_modifier(active: &mut ActiveStatModifiers, instance: StatModifierInstance) {
    let Some(existing) = active
        .modifiers
        .iter_mut()
        .find(|modifier| has_same_modifier_signature(modifier, &instance))
    else {
        active.modifiers.push(instance);
        return;
    };

    existing.remaining_seconds = instance.remaining_seconds;
}

/// Compares the gameplay identity of two modifiers while ignoring runtime-only
/// timer state.
///
/// HoT/DoT effects keep per-instance tick accumulators, so equality cannot be
/// used directly when deciding whether an incoming modifier should refresh an
/// existing one.
///
/// # Example
/// ```rust,ignore
/// assert!(has_same_modifier_signature(&old_swift, &new_swift));
/// ```
fn has_same_modifier_signature(left: &StatModifierInstance, right: &StatModifierInstance) -> bool {
    left.source == right.source
        && left.kind == right.kind
        && left.effects.len() == right.effects.len()
        && left
            .effects
            .iter()
            .zip(right.effects.iter())
            .all(|(left_effect, right_effect)| has_same_effect_signature(left_effect, right_effect))
}

/// Matches modifier effects by stat field and magnitude, excluding mutable tick
/// accumulators used by periodic effects.
///
/// # Example
/// ```rust,ignore
/// assert!(has_same_effect_signature(&left_effect, &right_effect));
/// ```
fn has_same_effect_signature(
    left: &ModifierEffectInstance,
    right: &ModifierEffectInstance,
) -> bool {
    match (left, right) {
        (
            ModifierEffectInstance::Stat {
                field: left_field,
                operation: left_operation,
                value: left_value,
            },
            ModifierEffectInstance::Stat {
                field: right_field,
                operation: right_operation,
                value: right_value,
            },
        ) => {
            left_field == right_field
                && left_operation == right_operation
                && are_modifier_values_equal(*left_value, *right_value)
        }
        (
            ModifierEffectInstance::HealOverTime {
                amount_per_tick: left_amount,
                tick_interval: left_interval,
                time_since_last_tick: _,
            },
            ModifierEffectInstance::HealOverTime {
                amount_per_tick: right_amount,
                tick_interval: right_interval,
                time_since_last_tick: _,
            },
        ) => {
            are_modifier_values_equal(*left_amount, *right_amount)
                && are_modifier_values_equal(*left_interval, *right_interval)
        }
        (
            ModifierEffectInstance::DamageOverTime {
                amount_per_tick: left_amount,
                tick_interval: left_interval,
                time_since_last_tick: _,
            },
            ModifierEffectInstance::DamageOverTime {
                amount_per_tick: right_amount,
                tick_interval: right_interval,
                time_since_last_tick: _,
            },
        ) => {
            are_modifier_values_equal(*left_amount, *right_amount)
                && are_modifier_values_equal(*left_interval, *right_interval)
        }
        _ => false,
    }
}

/// Compares gameplay tuning floats with a tiny tolerance to avoid direct float equality.
///
/// Modifier values are constants authored by gameplay code, but using a small
/// epsilon keeps the signature matcher robust if values are later loaded from
/// config or persistence.
///
/// # Example
/// ```rust,ignore
/// assert!(are_modifier_values_equal(1.2, 1.2));
/// ```
fn are_modifier_values_equal(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON
}

/// Decrements the duration of active modifiers and removes expired ones.
pub fn tick_stat_modifiers(
    time: Res<Time>,
    mut targets: Query<(Entity, &mut ActiveStatModifiers)>,
    mut heals: MessageWriter<HealEvent>,
    mut damages: MessageWriter<DamageEvent>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, mut active) in targets.iter_mut() {
        active.modifiers.retain_mut(|modifier| {
            let mut keep = true;
            if let Some(remaining) = modifier.remaining_seconds.as_mut() {
                *remaining -= delta;
                keep = *remaining > 0.0;
            }

            for effect in &mut modifier.effects {
                match effect {
                    ModifierEffectInstance::Stat { .. } => {}
                    ModifierEffectInstance::HealOverTime {
                        amount_per_tick,
                        tick_interval,
                        time_since_last_tick,
                    } => {
                        *time_since_last_tick += delta;
                        while *time_since_last_tick >= *tick_interval {
                            *time_since_last_tick -= *tick_interval;
                            heals.write(HealEvent {
                                target: entity,
                                source: modifier.source,
                                amount: *amount_per_tick,
                            });
                        }
                    }
                    ModifierEffectInstance::DamageOverTime {
                        amount_per_tick,
                        tick_interval,
                        time_since_last_tick,
                    } => {
                        *time_since_last_tick += delta;
                        while *time_since_last_tick >= *tick_interval {
                            *time_since_last_tick -= *tick_interval;
                            damages.write(DamageEvent {
                                target: entity,
                                source: modifier.source,
                                amount: *amount_per_tick,
                            });
                        }
                    }
                }
            }

            keep
        });
    }
}

/// Calculates the effective value of a `StatField` given the base value and active
/// modifiers. Order: all `Add` first, then `Multiply`,
/// and finally an `Override` wins over everything.
pub fn effective_value(field: StatField, base: f32, modifiers: &[StatModifierInstance]) -> f32 {
    let mut result = base;
    let mut override_value: Option<f32> = None;

    for modifier in modifiers {
        for effect in &modifier.effects {
            if let ModifierEffectInstance::Stat {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_value_applies_adds_then_multiplies() {
        let modifiers = vec![
            StatModifierInstance {
                id: ModifierId(0),
                source: None,
                effects: vec![ModifierEffectInstance::Stat {
                    field: StatField::Armor,
                    operation: ModifierOp::Add,
                    value: 20.0,
                }],
                remaining_seconds: None,
                kind: ModifierKind::Buff,
            },
            StatModifierInstance {
                id: ModifierId(1),
                source: None,
                effects: vec![ModifierEffectInstance::Stat {
                    field: StatField::Armor,
                    operation: ModifierOp::Multiply,
                    value: 1.5,
                }],
                remaining_seconds: None,
                kind: ModifierKind::Buff,
            },
        ];

        // base 10 + 20 = 30, * 1.5 = 45
        assert_eq!(effective_value(StatField::Armor, 10.0, &modifiers), 45.0);
    }

    #[test]
    fn effective_value_override_wins() {
        let modifiers = vec![StatModifierInstance {
            id: ModifierId(0),
            source: None,
            effects: vec![ModifierEffectInstance::Stat {
                field: StatField::Speed,
                operation: ModifierOp::Override,
                value: 99.0,
            }],
            remaining_seconds: None,
            kind: ModifierKind::Debuff,
        }];

        assert_eq!(effective_value(StatField::Speed, 0.15, &modifiers), 99.0);
    }
}
