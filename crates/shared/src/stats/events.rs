//! Stats events: damage, heal, and modifier (buff/debuff).
//!
//! These events are the channel through which spells, abilities, traps, and gear
//! communicate _what_ should happen. Systems in [`crate::stats::systems`]
//! decide _how_ to apply it (clamping, armor reduction, expiration, etc.).

use bevy::ecs::entity::Entity;
use bevy::prelude::Message;

/// Stat field targetable by a modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StatField {
    Speed,
    Armor,
    AttackPower,
    MaxHealth,
    ManaRegeneration,
}

/// Operation applied by the modifier to the base value.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ModifierOp {
    /// `base + value`
    Add,
    /// `base * value`
    Multiply,
    /// Temporarily overrides the base value with `value`.
    Override,
}

/// Narrative classification of the modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModifierKind {
    Buff,
    Debuff,
}

use bevy::prelude::Event;

/// Request to inflict `amount` damage on `target`.
///
/// Damage is intended as a _raw_ value, before armor reduction.
/// The [`crate::stats::systems::apply_damage`] system applies the armor
/// reduction formula and clamps health.
#[derive(Debug, Clone, PartialEq, Message, Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub amount: f32,
}

/// Request to heal `target` by `amount`.
///
/// Heal is clamped to `VitalStats.max_health`.
#[derive(Debug, Clone, PartialEq, Message, Event)]
pub struct HealEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub amount: f32,
}

/// A single effect of a modifier.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ModifierEffect {
    Stat {
        field: StatField,
        operation: ModifierOp,
        value: f32,
    },
    HealOverTime {
        amount_per_tick: f32,
        tick_interval: f32,
    },
    DamageOverTime {
        amount_per_tick: f32,
        tick_interval: f32,
    },
}

/// Request to apply a modifier (buff or debuff) to a `target`.
///
/// Modifiers are temporary: the
/// [`crate::stats::systems::tick_stat_modifiers`] system decrements duration and
/// removes expired modifiers.
#[derive(Debug, Clone, PartialEq, Message)]
pub struct ApplyStatModifierEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub effects: Vec<ModifierEffect>,
    /// `None` = permanent until explicitly removed.
    pub duration_seconds: Option<f32>,
    pub kind: ModifierKind,
}
