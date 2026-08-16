//! Runtime types for temporary modifiers (buffs/debuffs).

use crate::stats::events::{ModifierKind, ModifierOp, StatField};
use crate::EntityId;

/// Unique identifier for an applied modifier, for stacking management
/// and explicit removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModifierId(pub u64);

/// Runtime state of a single effect.
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

/// Instance of an active modifier on an entity.
#[derive(Debug, Clone, PartialEq)]
pub struct StatModifierInstance {
    pub id: ModifierId,
    pub source: Option<EntityId>,
    pub effects: Vec<ModifierEffectInstance>,
    /// `None` = permanent until explicitly removed.
    pub remaining_seconds: Option<f32>,
    pub kind: ModifierKind,
}

/// Component collecting all active modifiers on an entity.
///
/// Not persisted: modifiers are transient. Upon reconnection the player
/// restarts from base stats saved in the DB.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Default, Debug)]
pub struct ActiveStatModifiers {
    pub modifiers: Vec<StatModifierInstance>,
}
