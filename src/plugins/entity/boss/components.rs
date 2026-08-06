//! Boss-specific components for the dragon encounter.
//!
//! `Boss`, `BossPhase` and `BossArena` are replicated so clients can render the
//! arena ring, the boss bar and the phase banner. `ThreatTable`, `BossSpellbook`
//! and `BossRotationState` are server-only: they drive authoritative AI and never
//! cross the network.

use std::collections::HashMap;
use std::ops::AddAssign;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::spells::SpellId;

/// Marker for the dragon boss (Vermithrax, the Ashen Drake).
///
/// Distinguished from generic `Enemy` so the boss keeps its own AI, phase
/// machine and spellbook without inheriting the enemy respawn loop.
#[derive(Component, Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Boss;

/// Encounter phase. Replicated so the client can render a phase banner and
/// restyle the boss bar.
///
/// Transitions are server-decided (HP thresholds + enrage timer) and written
/// from `boss/systems.rs::update_boss_phase`.
#[derive(
    Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq, Default,
)]
#[reflect(Component)]
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
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
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
/// Server-only: keyed by player `Entity`, grown by `accrue_threat` listening to
/// `DamageEvent`s whose target is the boss. `Entity` keys are server-local and
/// never serialized, so this intentionally does not derive `Serialize`.
#[derive(Component, Debug, Default)]
pub struct ThreatTable {
    pub entries: HashMap<Entity, f32>,
}

impl ThreatTable {
    /// Adds `amount` of threat from `source`, creating the entry if new.
    pub fn add(&mut self, source: Entity, amount: f32) {
        self.entries.entry(source).or_insert(0.0).add_assign(amount);
    }
}

/// Boss ability set, bypassing the 3-slot player hotbar.
///
/// `process_cast_requests` treats a spell as castable if it is in the player
/// hotbar OR in this boss-only spellbook, so the dragon can cycle more than
/// three abilities. Server-only.
#[derive(Component, Debug, Clone, Default)]
pub struct BossSpellbook {
    pub spells: Vec<SpellId>,
}

impl BossSpellbook {
    /// Returns true if the boss knows the given spell.
    pub fn contains(&self, spell_id: &SpellId) -> bool {
        self.spells.iter().any(|known| known == spell_id)
    }
}

/// Per-boss scheduler state for the ability rotation.
///
/// Server-only. Drives the priority cursor and the enrage timer
/// (`engaged_seconds` vs `BERSERK_TIMER_SECONDS`).
#[derive(Component, Debug, Default)]
pub struct BossRotationState {
    /// Seconds since the encounter engaged; gates the hard enrage timer.
    pub engaged_seconds: f32,
    /// Cursor into the current phase's priority list.
    pub priority_cursor: usize,
}
