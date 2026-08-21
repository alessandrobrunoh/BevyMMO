//! Boss-specific components for the dragon encounter.
//!
//! `Boss`, `BossPhase` and `BossArena` are replicated so clients can render the
//! arena ring, the boss bar and the phase banner. `ThreatTable` and
//! `BossRotationState` are server-only.

// `#[reflect(Component)]` expands to a reference to this type.
#[cfg(feature = "bevy")]
use bevy_ecs::reflect::ReflectComponent;

use crate::EntityId;
use glam::Vec3;
use std::collections::HashMap;
use std::ops::AddAssign;

use serde::{Deserialize, Serialize};

/// Marker for the dragon boss (Vermithrax, the Ashen Drake).
///
/// Distinguished from generic `Enemy` so the boss keeps its own AI, phase
/// machine and spellbook without inheriting the enemy respawn loop.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Boss;

/// Encounter phase. Replicated so the client can render a phase banner and
/// restyle the boss bar.
///
/// Transitions are server-decided (HP thresholds + enrage timer) and written
/// from `boss/systems.rs::update_boss_phase`.
#[cfg_attr(
    feature = "bevy",
    derive(bevy_ecs::component::Component, bevy_reflect::Reflect)
)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum BossPhase {
    /// Before any player enters the arena ring. Boss is idle and ignored by AI.
    #[default]
    Dormant,
    /// Phase 1 (100% - 66% HP): grounded melee + breath rotation.
    Ground,
    /// Phase 2 (66% - 33% HP): aerial, arena-wide eruption patterns.
    Aerial,
    /// Phase 3 (33% - 0% HP) or forced enrage: cast haste + Cataclysm.
    Berserk,
    /// Terminal: boss defeated, stays as a corpse (no auto-respawn).
    Dead,
}

/// Arena trigger ring, anchored to the boss spawn.
///
/// The server flips `is_engaged` to true the first time a living player crosses
/// `radius` around `center`; it never resets in v1. Clients read the replicated
/// component to draw the pulsing red ring and fade it on engage.
#[cfg_attr(
    feature = "bevy",
    derive(bevy_ecs::component::Component, bevy_reflect::Reflect)
)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct BossArena {
    /// Fixed world position of the arena center (equals the boss `SpawnPoint`).
    pub center: Vec3,
    /// Radius a player must enter to start the encounter.
    pub radius: f32,
    /// True once the encounter has started; never goes back to false in v1.
    pub is_engaged: bool,
}

/// Threat accrued by players damaging the boss.
///
/// Server-only: keyed by player `EntityId`, grown by `accrue_threat` listening to
/// `DamageEvent`s whose target is the boss. `EntityId` keys are server-local and
/// never serialized, so this intentionally does not derive `Serialize`.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default)]
pub struct ThreatTable {
    pub entries: HashMap<EntityId, f32>,
}

impl ThreatTable {
    /// Adds `amount` of threat from `source`, creating the entry if new.
    pub fn add(&mut self, source: EntityId, amount: f32) {
        self.entries.entry(source).or_insert(0.0).add_assign(amount);
    }
}

/// Per-boss scheduler state for the ability rotation.
///
/// Server-only. Drives the priority cursor and the enrage timer
/// (`engaged_seconds` vs `BERSERK_TIMER_SECONDS`).
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default)]
pub struct BossRotationState {
    /// Seconds since the encounter engaged; gates the hard enrage timer.
    pub engaged_seconds: f32,
    /// Cursor into the current phase's priority list.
    pub priority_cursor: usize,
}

impl Boss {
    /// Radius of the arena trigger ring centered on the boss spawn.
    pub const ARENA_RADIUS: f32 = 12.0;
}
