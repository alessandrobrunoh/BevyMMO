//! Spell framework data: the `Spell` trait, cast context, registry and
//! runtime components.
//!
//! The cast pipeline (processing requests, advancing cast progress, firing
//! effects) is server logic and lives in `bevymmo_server`. This crate only
//! defines the contract and the data.

pub mod components;
pub mod context;
pub mod events;
pub mod registry;

pub use components::{
    default_player_hotbar, CastProgress, HotbarSlot, SpellCooldowns, SpellHotbar,
};
pub use context::{
    AoeEffect, AoeShape, AoeSpawnRequest, AoeTargeting, CastKind, ChannelMovementPolicy,
    ProjectileSpawnRequest, Spell, SpellCastContext, SpellConfig, TargetingMode,
};
pub use events::{SpellCastRequest, SpellReleaseRequest};
pub use registry::{SpellId, SpellRegistry};
