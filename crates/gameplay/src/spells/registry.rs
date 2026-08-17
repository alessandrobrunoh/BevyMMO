//! Central registry of all available spells.
//!
//! Spells are registered at startup and looked up by ID during cast processing.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

use super::context::Spell;
use crate::registry::Registry;

/// Unique identifier for a spell type.
///
/// Uses `Cow<'static, str>` to allow both borrowed (static) and owned strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpellId(pub(crate) Cow<'static, str>);

impl SpellId {
    /// Create a new SpellId from either a static string or an owned network string.
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    /// Get the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for SpellId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

/// Central registry of all available spells.
///
/// Spells are registered at startup and looked up by ID during cast processing.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
#[derive(Default)]
pub struct SpellRegistry {
    spells: Registry<SpellId, Arc<dyn Spell>>,
}

impl SpellRegistry {
    /// Register a spell in the registry.
    ///
    /// If a spell with the same ID already exists, it will be replaced.
    pub fn register(&mut self, spell: Arc<dyn Spell>) {
        let id = spell.id();
        self.spells.insert(id, spell);
    }

    /// Look up a spell by its ID.
    ///
    /// Returns `None` if no spell with the given ID is registered.
    pub fn get(&self, id: &SpellId) -> Option<Arc<dyn Spell>> {
        self.spells.get(id).cloned()
    }

    /// Get the number of registered spells.
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }

    /// Check if a spell exists in the registry.
    pub fn contains(&self, id: &SpellId) -> bool {
        self.spells.contains(id)
    }

    /// Get all registered spells, sorted alphabetically by display name.
    pub fn sorted_spells(&self) -> Vec<(SpellId, Arc<dyn Spell>)> {
        self.spells
            .sorted_by(|a, b| a.display_name().cmp(b.display_name()))
    }
}
