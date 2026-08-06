//! Tipi runtime per i modifier temporanei (buff/debuff).

use crate::stats::events::{ModifierKind, ModifierOp, StatField};
use bevy::prelude::*;

/// Identificatore univoco di un modifier applicato, per gestione stacking
/// e rimozione esplicita.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModifierId(pub u64);

/// Stato a runtime di un singolo effetto.
#[derive(Debug, Clone, PartialEq)]
pub enum ModifierEffectInstance {
    Stat {
        field: StatField,
        operation: ModifierOp,
        value: f32,
    },
    HealOverTime {
        amount_per_tick: f32,
        tick_interval: f32,
        time_since_last_tick: f32,
    },
    DamageOverTime {
        amount_per_tick: f32,
        tick_interval: f32,
        time_since_last_tick: f32,
    },
}

/// Istanza di un modifier attivo su un'entità.
#[derive(Debug, Clone, PartialEq)]
pub struct StatModifierInstance {
    pub id: ModifierId,
    pub source: Option<Entity>,
    pub effects: Vec<ModifierEffectInstance>,
    /// `None` = permanente finché non rimosso esplicitamente.
    pub remaining_seconds: Option<f32>,
    pub kind: ModifierKind,
}

/// Componente che collezione tutti i modifier attivi su un'entità.
///
/// Non è persistita: i modifier sono transient. Alla roconnessione il player
/// riparte dalle stat base salvate nel DB.
#[derive(Component, Default, Debug)]
pub struct ActiveStatModifiers {
    pub modifiers: Vec<StatModifierInstance>,
}
