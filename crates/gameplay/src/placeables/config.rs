//! Concrete configuration DTOs returned by the category subtraits.
//!
//! These are **not** ECS components — they are plain data passed from the
//! definition trait to the spawn machinery. Keeping them as plain structs
//! (instead of returning `impl Bundle`) preserves object safety of the
//! traits, so the registry can store `Arc<dyn EnemyPlaceable>` and dispatch
//! dynamically without recompiling per kind.

use crate::abilities::{AbilityId, KitInscription};
use crate::items::definition::ItemCategory;
use crate::spells::SpellId;
use crate::stats::components::StatsBundleData;

// -------------------------------------------------------------------------
// Creature configs
// -------------------------------------------------------------------------

/// One ability on an enemy kit: a shared `BaseAbility` plus an optional
/// content-authored inscription (no player glyph gate).
#[derive(Debug, Clone)]
pub struct AbilityKitEntry {
    pub ability_id: AbilityId,
    pub inscription: KitInscription,
}

impl AbilityKitEntry {
    /// Naked gesture, no Root Word.
    pub fn new(ability_id: impl Into<AbilityId>) -> Self {
        Self {
            ability_id: ability_id.into(),
            inscription: KitInscription::default(),
        }
    }
}

/// Configuration returned by [`super::definition::EnemyPlaceable::enemy_config`].
///
/// Drives spawn: stats, aggro acquire radius, leash-from-spawn, and the
/// ability kit the AI fires through `resolve_ability`.
#[derive(Debug, Clone)]
pub struct EnemyConfig {
    /// Stat profile for this archetype (HP, mana, armor, attack, speed).
    pub stats: StatsBundleData,
    /// Acquire: a living player inside this radius of the mob enters combat.
    pub aggro: f32,
    /// Reset: if the mob is farther than this from its spawn, drop and go home.
    pub leash_aggro: f32,
    /// Shared `BaseAbility` ids, in picker order. Slice 1 uses the first entry.
    pub abilities: Vec<AbilityKitEntry>,
}

/// Configuration returned by [`super::definition::BossPlaceable::boss_config`].
///
/// The boss plugin reads the `rotation` list to build the
/// `BossSpellbook`; today that list is hardcoded as `Boss::SPELLS` in the
/// dragon definition — this DTO replaces that constant so each boss kind
/// can declare its own rotation.
#[derive(Debug, Clone)]
pub struct BossConfig {
    /// Stat profile for this boss.
    pub stats: StatsBundleData,
    /// Spell ids in the boss rotation, in priority order.
    pub rotation: Vec<SpellId>,
    /// Radius of the arena trigger centered on the boss spawn.
    pub arena_radius: f32,
}

// -------------------------------------------------------------------------
// Interaction configs
// -------------------------------------------------------------------------

/// Interaction kind returned by `NpcPlaceable` and `InteractablePlaceable`.
///
/// Kept as a non-`Component` enum so the trait stays object-safe; the server
/// binding converts it into the appropriate replicated component at spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionKind {
    /// Opens a shop inventory. `inventory_id` references a content table.
    Shop { inventory_id: String },
    /// Opens an isolated player market. `market_id` is `market_1` / `market_2`.
    Market { market_id: String },
    /// Opens a dialogue tree. `dialogue_tree_id` references a dialogue asset.
    Dialogue { dialogue_tree_id: String },
    /// Opens a crafter UI listing catalogue items in `category` that have a recipe.
    Craft { category: ItemCategory },
    /// Opens a chest and rolls the given loot table.
    OpenChest { loot_table_id: String },
    /// Toggles a door (open / closed).
    OpenDoor,
}

// -------------------------------------------------------------------------
// Trigger configs
// -------------------------------------------------------------------------

/// Configuration returned by [`super::definition::TriggerPlaceable::trigger_config`].
#[derive(Debug, Clone)]
pub struct TriggerConfig {
    /// 2D shape projected onto the XZ plane.
    pub shape: TriggerShape,
    /// What happens when an entity enters / leaves the shape.
    pub event: TriggerEvent,
    /// If true, the event fires at most once per entity per map session.
    pub once_per_entity: bool,
}

/// 2D trigger shape, ignoring Y (triggers are vertical prisms).
#[derive(Debug, Clone)]
pub enum TriggerShape {
    /// Cylinder on the Y axis.
    Circle { radius: f32 },
    /// Axis-aligned box on the XZ plane.
    Box { half_extents: [f32; 2] },
}

/// Effect fired by a trigger when its activation condition is met.
#[derive(Debug, Clone)]
pub enum TriggerEvent {
    /// Marks the inside region as PvP-enabled.
    EnterPvpZone,
    /// Marks the inside region as safe (no combat).
    EnterSafeZone,
    /// Teleports the entering entity to another map / position.
    Teleport {
        target_map: String,
        target_position: [f32; 3],
    },
}

// -------------------------------------------------------------------------
// Resource configs
// -------------------------------------------------------------------------

/// Configuration returned by
/// [`super::definition::ResourceNodePlaceable::resource_config`].
#[derive(Debug, Clone)]
pub struct ResourceConfig {
    /// Maximum pieces the node can hold.
    pub max_pieces: u32,
    /// Base seconds to channel one piece, before gathering speed.
    pub channel_seconds: f32,
    /// Floor on channel duration (anti-exploit).
    pub min_channel_seconds: f32,
    /// Item granted on a completed channel.
    pub yield_item: crate::items::ItemId,
    /// Base pieces granted per completed channel.
    pub yield_amount: u32,
    /// Seconds between regen pulses.
    pub regen_interval_seconds: f32,
    /// Pieces restored each regen pulse.
    pub regen_amount: u32,
    /// Horizontal gather range in world units.
    pub interact_range: f32,
    /// Optional tool required to start a gather. `None` in v1.
    pub required_item_id: Option<crate::items::ItemId>,
}
