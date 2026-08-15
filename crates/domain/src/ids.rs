//! Stable identifiers for simulation entities.

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
