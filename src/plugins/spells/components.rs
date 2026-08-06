//! ECS components for spell-related runtime state.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::registry::SpellId;

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
