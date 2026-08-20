//! Concrete configuration DTOs returned by the category subtraits.
//!
//! These are **not** ECS components — they are plain data passed from the
//! definition trait to the spawn machinery. Keeping them as plain structs
//! (instead of returning `impl Bundle`) preserves object safety of the
//! traits, so the registry can store `Arc<dyn EnemyPlaceable>` and dispatch
//! dynamically without recompiling per kind.

use crate::spells::{SpellHotbar, SpellId};
use crate::stats::components::StatsBundleData;

// -------------------------------------------------------------------------
// Creature configs
// -------------------------------------------------------------------------

/// Configuration returned by [`super::definition::EnemyPlaceable::enemy_config`].
///
/// Drives the `spawn_entity::<Enemy>()` override layer: the server spawns the
/// existing `Enemy` entity (which already wires stats / replication / AI) and
/// then overrides the configured stats, hotbar and aggro range with the
/// per-kind values defined here.
#[derive(Debug, Clone)]
pub struct EnemyConfig {
    /// Stat profile for this archetype (HP, attack, armor, speed).
    pub stats: StatsBundleData,
    /// Spells available to the archetype. Today enemies only use the `Q` slot.
    pub spell_hotbar: SpellHotbar,
    /// Distance at which the enemy starts chasing a target.
    pub aggro_range: f32,
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
#[derive(Debug, Clone)]
pub enum InteractionKind {
    /// Opens a shop inventory. `inventory_id` references a content table.
    Shop { inventory_id: String },
    /// Opens an isolated player market. `market_id` is `market_1` / `market_2`.
    Market { market_id: String },
    /// Opens a dialogue tree. `dialogue_tree_id` references a dialogue asset.
    Dialogue { dialogue_tree_id: String },
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
    /// How much harvesting damage the node can take before depleting.
    pub max_health: f32,
    /// Seconds before the node respawns after depletion.
    pub respawn_seconds: f32,
    /// Item id yielded when harvested (resolved by the item system).
    pub yield_item: String,
    /// Quantity yielded per harvest tick.
    pub yield_amount: u32,
}
