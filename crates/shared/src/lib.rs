//! Shared core types for the BevyMMO workspace.
//!
//! This crate holds everything that `server`, `client`, `presentation` and
//! `editor` must agree on: network protocol types, gameplay components, stats
//! data, and the world manifest format. It is intentionally role-agnostic:
//! it contains no sockets, no rendering, and no editor tooling.
//!
//! Dependency rule: nothing in this crate may depend on the other workspace
//! crates. See `plans/workspace-crate-split.md` (D1).

pub mod crowd_control;
pub mod entity;
pub mod game_state;
pub mod items;
pub mod items_impl;
pub mod movement;
pub mod network;
pub mod paths;
pub mod placeables;
pub mod placeables_impl;
pub mod settings;
pub mod spells;
pub mod spells_impl;
pub mod stats;
pub mod targeting;
pub mod world;

pub use crate::movement::{ClientSurfaceQuery, MoveTarget};

/// Common re-exports for consumers of this crate.
pub mod prelude {
    pub use crate::crowd_control::{ActiveCrowdControl, CrowdControlKind, CrowdControlState};
    pub use crate::entity::components::{
        EntityKind, EntityState, GameEntity, PlayerName, SpawnPoint,
    };
    pub use crate::entity::definition::EntityDefinition;
    pub use crate::entity::events::{DeathEvent, RespawnedEvent};
    pub use crate::entity::spawn::{spawn_entity, GameEntityBundle};
    pub use crate::items::events::{EquipItemCommand, MoveItemCommand, UnequipItemCommand};
    pub use crate::items::{
        EquipSlot, Equipment, Inventory, Item, ItemCategory, ItemConfig, ItemEffect, ItemId,
        ItemRarity, ItemRegistry, INVENTORY_CAPACITY,
    };
    pub use crate::network::mode::{has_client, has_server, AppMode};
    pub use crate::network::protocol::{
        EntityColor, Inputs, LookDirection, NetworkEntityId, PlayerId, Position, ProjectileVisual,
        SpellCastCommand, SpellCastEnded, SpellCastProgress, SpellCastRelease, SpellVisualEffect,
        UpdateHotbarSlotRequest,
    };
    pub use crate::settings::Settings;
    pub use crate::spells::{
        default_player_hotbar, AoeEffect, AoeTargeting, CastKind, CastProgress,
        ChannelMovementPolicy, HotbarSlot, ProjectileSpawnRequest, Spell, SpellCastContext,
        SpellCastRequest, SpellConfig, SpellCooldowns, SpellHotbar, SpellId, SpellRegistry,
        SpellReleaseRequest, TargetingMode,
    };
    pub use crate::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};
    pub use crate::stats::events::{
        ApplyStatModifierEvent, DamageEvent, HealEvent, ModifierEffect, ModifierKind, ModifierOp,
        StatField,
    };
    pub use crate::targeting::CurrentTarget;
}
