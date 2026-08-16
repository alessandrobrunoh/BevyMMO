//! Top-level classification of placeable kinds.
//!
//! This enum is a **UI hint only** — it is used by the editor palette to
//! group entries. Runtime dispatch never uses it: dispatch is done via
//! the category subtraits (`EnemyPlaceable`, `BossPlaceable`, ...), so
//! the server picks the right spawn machinery by looking up the typed
//! submap. Keeping this enum purely cosmetic means adding a new kind
//! never requires editing a central `match`.

use serde::{Deserialize, Serialize};

/// Palette grouping for the editor. The server and client do not branch on
/// this value; they branch on which typed subtrait a definition implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaceableCategory {
    /// Static visual props (trees, rocks, houses). No behavior.
    Prop,
    /// Anything that spawns a gameplay entity: player spawn, mobs, bosses, NPCs.
    Creature,
    /// Invisible gameplay zones (PvP, teleport, area triggers).
    Trigger,
    /// Harvestable nodes (ore veins, herbs).
    ResourceNode,
    /// One-shot interactions (doors, levers, chests).
    Interactable,
}

impl PlaceableCategory {
    /// All categories in palette display order.
    pub const ALL: [Self; 5] = [
        Self::Prop,
        Self::Creature,
        Self::Trigger,
        Self::ResourceNode,
        Self::Interactable,
    ];

    /// Human-readable label shown in the editor tab strip.
    pub fn label(self) -> &'static str {
        match self {
            Self::Prop => "Props",
            Self::Creature => "Creatures",
            Self::Trigger => "Triggers",
            Self::ResourceNode => "Resources",
            Self::Interactable => "Interactables",
        }
    }
}
