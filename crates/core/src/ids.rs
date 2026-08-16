//! Stable identifiers for simulation entities.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Identifies a simulation entity across the client/server boundary.
///
/// This replaces `bevy::ecs::entity::Entity` in every piece of game logic that
/// has to run inside the SpacetimeDB module, where Bevy's ECS does not exist.
/// On the server it is the `game_entity.entity_id` column; on the client it is
/// resolved back to a Bevy `Entity` through the row-to-entity map.
///
/// It deliberately does *not* derive `Default`: a zero id is not a valid entity,
/// and `#[auto_inc]` columns treat zero as "assign me one", so a silently
/// defaulted `EntityId` would be a bug that only shows up at insert time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

impl EntityId {
    /// A stand-in for code that must build a value it will never look up —
    /// previewing a spell's shape client-side, for instance. Mirrors
    /// `bevy::ecs::entity::Entity::PLACEHOLDER`.
    pub const PLACEHOLDER: Self = Self(u64::MAX);

    /// Wraps a raw database id.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The underlying database id.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl core::fmt::Display for EntityId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "e{}", self.0)
    }
}

/// Stable unique identifier for a placeable kind.
///
/// Stored in world manifests and resolved against the gameplay placeable
/// registry at runtime. It belongs to core so map data does not depend on that
/// registry's gameplay-specific configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KindId(Cow<'static, str>);

impl KindId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for KindId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl core::fmt::Display for KindId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
