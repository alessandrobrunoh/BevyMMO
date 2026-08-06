//! ECS components for spell-related runtime state.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::context::{CastKind, ChannelMovementPolicy};
use super::registry::SpellId;

const DEFAULT_PLAYER_SPELL_IDS: [&str; 6] = [
    "attack",
    "fireball",
    "healing_circle",
    "meteorite",
    "stun_field",
    "swift",
];

/// Authoritative initial player spellbook.
///
/// Keeping the defaults in one function avoids drift between first-time player
/// creation, database backfills, and non-persisted player spawns.
///
/// # Example
/// ```rust
/// let spellbook = default_player_spellbook();
/// assert!(spellbook.contains(&SpellId::new("attack")));
/// ```
pub fn default_player_spellbook() -> Spellbook {
    Spellbook::from_ids(DEFAULT_PLAYER_SPELL_IDS.map(SpellId::new))
}

// TODO: QUesto sará da modificare e rimuovere perché le spell sono attaccate agli Enemy o agli Items e non anche ai Player.
/// A collection of spells known to an entity.
///
/// This component represents which spells an entity (typically a player) has access to.
#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Spellbook {
    /// List of spell IDs that this entity can cast.
    pub spells: Vec<SpellId>,
}

impl Spellbook {
    /// Create a spellbook with a single spell.
    pub fn single(id: SpellId) -> Self {
        Self { spells: vec![id] }
    }

    /// Create a spellbook from a list of spell ids.
    pub fn from_ids(spells: impl IntoIterator<Item = SpellId>) -> Self {
        Self {
            spells: spells.into_iter().collect(),
        }
    }

    /// Create an empty spellbook.
    pub fn empty() -> Self {
        Self { spells: Vec::new() }
    }

    /// Check if the spellbook contains a specific spell.
    pub fn contains(&self, id: &SpellId) -> bool {
        self.spells.contains(id)
    }

    /// Add a spell to the spellbook if it's not already present.
    pub fn add(&mut self, id: SpellId) {
        if !self.contains(&id) {
            self.spells.push(id);
        }
    }

    /// Remove a spell from the spellbook.
    pub fn remove(&mut self, id: &SpellId) -> bool {
        if let Some(pos) = self.spells.iter().position(|s| s == id) {
            self.spells.remove(pos);
            true
        } else {
            false
        }
    }
}

/// Cooldown timers for spells that have been cast.
///
/// This component tracks remaining cooldown time for each spell cast by an entity.
#[derive(Component, Debug, Default)]
pub struct SpellCooldowns {
    /// Map of spell ID to remaining cooldown timer.
    pub timers: HashMap<SpellId, Timer>,
}

impl SpellCooldowns {
    /// Get the remaining cooldown time for a spell.
    ///
    /// Returns `None` if the spell is not on cooldown.
    pub fn get_remaining(&self, id: &SpellId) -> Option<f32> {
        self.timers.get(id).map(|timer| timer.elapsed_secs())
    }

    /// Check if a spell is currently on cooldown.
    pub fn is_on_cooldown(&self, id: &SpellId) -> bool {
        self.timers
            .get(id)
            .is_some_and(|timer| !timer.is_finished())
    }

    /// Start a cooldown for a spell.
    pub fn start_cooldown(&mut self, id: SpellId, duration_seconds: f32) {
        self.timers
            .insert(id, Timer::from_seconds(duration_seconds, TimerMode::Once));
    }

    /// Tick all cooldown timers.
    pub fn tick(&mut self, delta: std::time::Duration) {
        for timer in self.timers.values_mut() {
            timer.tick(delta);
        }
    }

    /// Clean up finished cooldown timers to free memory.
    pub fn cleanup_finished(&mut self) {
        self.timers.retain(|_, timer| !timer.is_finished());
    }
}

/// Soglia di spostamento orizzontale (in unità) oltre la quale un cast-time
/// o un channeling `InterruptOnMove` viene cancellato. Tunable.
pub const MOVEMENT_INTERRUPT_EPSILON: f32 = 0.05;

/// Stato server-authoritative di una spell in fase di cast (CastTime) o
/// channeling. Il sistema [`crate::plugins::spells::systems::advance_cast_progress`]
/// lo ticka ogni frame e decide quando lanciare l'effetto.
///
/// Una sola istanza per caster: starting una nuova spell mentre questo
/// componente esiste cancella quella precedente.
#[derive(Component, Debug)]
pub struct CastProgress {
    pub spell_id: SpellId,
    pub kind: CastKind,
    pub elapsed_seconds: f32,
    /// Per `CastTime`: tempo richiesto prima del fire. Per `Channeling` è
    /// ignorato (open-ended).
    pub required_seconds: f32,
    /// Policy di interruzione col movimento, copiata dalla `SpellConfig` al
    /// momento dello spawn per evitare un lookup ad ogni tick.
    pub channel_movement: ChannelMovementPolicy,
    /// Snapshot della posizione del caster all'ultimo tick, per rilevare
    /// spostamenti che debbano interrompere il cast.
    pub last_position: Vec3,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<Entity>,
    /// Comando movimento attivo quando il cast è iniziato. Serve per ignorare
    /// il movimento precedente e interrompere solo su un nuovo click/input.
    pub movement_input_at_start: Option<Vec3>,
    /// Accumulatore per il tick interval del channeling. Quando supera
    /// `tick_interval_seconds` la spell viene ri-eseguita.
    pub channel_tick_accumulator_seconds: f32,
    pub tick_interval_seconds: f32,
}
