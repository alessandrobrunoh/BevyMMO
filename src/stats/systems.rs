//! Sistemi delle statistiche: applicazione danno/cura/modifier, scadenza
//! modifier e gestione morte.
//!
//! Tutti i sistemi che mutano lo stato di gameplay sono server-authoritative.

use bevy::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::stats::components::{CombatStats, VitalStats};
use crate::stats::events::{ApplyStatModifierEvent, DamageEvent, HealEvent, ModifierOp, StatField};
use crate::stats::formulas::damage_after_armor;
use crate::stats::modifiers::{ActiveStatModifiers, ModifierId, StatModifierInstance};

static NEXT_MODIFIER_ID: AtomicU64 = AtomicU64::new(0);

/// Applica i `DamageEvent` accumulati: riduzione armatura + clamp.
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

/// Applica i `HealEvent` accumulati, clamping al massimo.
pub fn apply_healing(mut events: MessageReader<HealEvent>, mut targets: Query<&mut VitalStats>) {
    for event in events.read() {
        let Ok(mut vital) = targets.get_mut(event.target) else {
            continue;
        };
        vital.current_health = (vital.current_health + event.amount).min(vital.max_health);
    }
}

/// Converte `ApplyStatModifierEvent` in `StatModifierInstance` attaccati
/// al bersaglio via `ActiveStatModifiers`.
pub fn apply_stat_modifiers(
    mut commands: Commands,
    mut events: MessageReader<ApplyStatModifierEvent>,
    mut targets: Query<&mut ActiveStatModifiers>,
) {
    for event in events.read() {
        let instance = StatModifierInstance {
            id: ModifierId(NEXT_MODIFIER_ID.fetch_add(1, Ordering::Relaxed)),
            source: event.source,
            field: event.field,
            operation: event.operation,
            value: event.value,
            remaining_seconds: event.duration_seconds,
            kind: event.kind,
        };

        match targets.get_mut(event.target) {
            Ok(mut active) => active.modifiers.push(instance),
            // L'entità non ha ancora `ActiveStatModifiers`: inseriscilo e riprova.
            _ => {
                commands.entity(event.target).insert(ActiveStatModifiers {
                    modifiers: vec![instance],
                });
            }
        }
    }
}

/// Decrementa la durata dei modifier attivi e rimuove quelli scaduti.
pub fn tick_stat_modifiers(time: Res<Time>, mut targets: Query<&mut ActiveStatModifiers>) {
    let delta = time.delta().as_secs_f32();
    for mut active in targets.iter_mut() {
        active.modifiers.retain_mut(|modifier| {
            if let Some(remaining) = modifier.remaining_seconds.as_mut() {
                *remaining -= delta;
                *remaining > 0.0
            } else {
                true
            }
        });
    }
}

/// Calcola il valore effective di un `StatField` dato il valore base e i
/// modifier attivi. L'ordine è: tutti gli `Add` prima, poi `Multiply`,
/// infine un eventuale `Override` vince su tutto.
pub fn effective_value(field: StatField, base: f32, modifiers: &[StatModifierInstance]) -> f32 {
    let mut result = base;
    let mut override_value: Option<f32> = None;

    for modifier in modifiers {
        if modifier.field != field {
            continue;
        }
        match modifier.operation {
            ModifierOp::Add => result += modifier.value,
            ModifierOp::Multiply => result *= modifier.value,
            ModifierOp::Override => override_value = Some(modifier.value),
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
                field: StatField::Armor,
                operation: ModifierOp::Add,
                value: 20.0,
                remaining_seconds: None,
                kind: crate::stats::events::ModifierKind::Buff,
            },
            StatModifierInstance {
                id: ModifierId(1),
                source: None,
                field: StatField::Armor,
                operation: ModifierOp::Multiply,
                value: 1.5,
                remaining_seconds: None,
                kind: crate::stats::events::ModifierKind::Buff,
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
            field: StatField::Speed,
            operation: ModifierOp::Override,
            value: 99.0,
            remaining_seconds: None,
            kind: crate::stats::events::ModifierKind::Debuff,
        }];

        assert_eq!(effective_value(StatField::Speed, 0.15, &modifiers), 99.0);
    }
}
