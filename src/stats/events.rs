//! Eventi delle statistiche: danno, cura e modifier (buff/debuff).
//!
//! Questi eventi sono il canale tramite cui spell, abilità, trap e gear
//! comunicano _cosa_ deve succedere. I sistemi in [`crate::stats::systems`]
//! decido _come_ applicarlo (clamp, armor reduction, expiration, ecc.).

use bevy::ecs::entity::Entity;
use bevy::prelude::Message;

/// Campo statistica bersagliabile da un modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StatField {
    Speed,
    Armor,
    AttackPower,
    MaxHealth,
    ManaRegeneration,
}

/// Operazione applicata dal modifier al valore base.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ModifierOp {
    /// `base + value`
    Add,
    /// `base * value`
    Multiply,
    /// Sostituisce temporaneamente il valore base con `value`.
    Override,
}

/// Classificazione narrativa del modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModifierKind {
    Buff,
    Debuff,
}

/// Richiesta di infliggere `amount` danno a `target`.
///
/// Il danno è inteso come valore _grezzo_, prima della riduzione da armatura.
/// Il sistema [`crate::stats::systems::apply_damage`] applica la formula di
/// armor reduction e clampa la salute.
#[derive(Debug, Clone, PartialEq, Message)]
pub struct DamageEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub amount: f32,
}

/// Richiesta di curare `target` di `amount`.
///
/// La cura viene clamped a `VitalStats.max_health`.
#[derive(Debug, Clone, PartialEq, Message)]
pub struct HealEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub amount: f32,
}

/// Richiesta di applicare un modifier (buff o debuff) a una stat di `target`.
///
/// I modifier sono temporanei: il sistema
/// [`crate::stats::systems::tick_stat_modifiers`] decrementa la durata e
/// rimuove i modifier scaduti.
#[derive(Debug, Clone, PartialEq, Message)]
pub struct ApplyStatModifierEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub field: StatField,
    pub operation: ModifierOp,
    pub value: f32,
    /// `None` = permanente finché non rimosso esplicitamente.
    pub duration_seconds: Option<f32>,
    pub kind: ModifierKind,
}
