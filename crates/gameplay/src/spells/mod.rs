//! Shared cast runtime: context, targeting, AoE/projectile spawn requests,
//! and the visual event that both `BaseAbility` and the module drain.
//!
//! Combat content lives in [`crate::abilities`]. There is no spell catalog.

pub mod components;
pub mod context;
pub mod visuals;

pub use components::CastProgress;
pub use context::{
    AoeShape, AoeSpawnRequest, AoeTargeting, CastKind, ChannelMovementPolicy,
    ProjectileSpawnRequest, SpellCastContext, TargetingMode,
};

pub use visuals::SpellVisualEffect;
