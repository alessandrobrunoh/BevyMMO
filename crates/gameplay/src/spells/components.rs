//! ECS components for spell-related runtime state.

use glam::Vec3;
use crate::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::context::{CastKind, ChannelMovementPolicy};
use super::registry::SpellId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HotbarSlot {
    Q,
    W,
    E,
}

#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SpellHotbar {
    pub q_spell: Option<SpellId>,
    pub w_spell: Option<SpellId>,
    pub e_spell: Option<SpellId>,
}

impl SpellHotbar {
    pub fn spell_for_slot(&self, slot: HotbarSlot) -> Option<&SpellId> {
        match slot {
            HotbarSlot::Q => self.q_spell.as_ref(),
            HotbarSlot::W => self.w_spell.as_ref(),
            HotbarSlot::E => self.e_spell.as_ref(),
        }
    }

    pub fn assign(&mut self, slot: HotbarSlot, spell_id: Option<SpellId>) {
        if let Some(id) = &spell_id {
            if self.q_spell.as_ref() == Some(id) {
                self.q_spell = None;
            }
            if self.w_spell.as_ref() == Some(id) {
                self.w_spell = None;
            }
            if self.e_spell.as_ref() == Some(id) {
                self.e_spell = None;
            }
        }

        match slot {
            HotbarSlot::Q => self.q_spell = spell_id,
            HotbarSlot::W => self.w_spell = spell_id,
            HotbarSlot::E => self.e_spell = spell_id,
        }
    }

    pub fn contains(&self, spell_id: &SpellId) -> bool {
        self.q_spell.as_ref() == Some(spell_id)
            || self.w_spell.as_ref() == Some(spell_id)
            || self.e_spell.as_ref() == Some(spell_id)
    }
}

pub fn default_player_hotbar() -> SpellHotbar {
    SpellHotbar {
        q_spell: Some(SpellId::new("attack")),
        w_spell: Some(SpellId::new("fireball")),
        e_spell: Some(SpellId::new("healing_circle")),
    }
}

/// A single countdown, in seconds.
///
/// Replaces `bevy::time::Timer`, which cannot follow this type into the
/// SpacetimeDB module. Deliberately minimal: elapsed and duration, ticked by an
/// explicit `f32` rather than a `Duration`, because the module's tick measures
/// its own delta and has no `Time` resource to ask.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cooldown {
    elapsed_seconds: f32,
    duration_seconds: f32,
}

impl Cooldown {
    pub fn new(duration_seconds: f32) -> Self {
        Self {
            elapsed_seconds: 0.0,
            duration_seconds,
        }
    }

    pub fn elapsed_secs(self) -> f32 {
        self.elapsed_seconds
    }

    pub fn remaining_secs(self) -> f32 {
        (self.duration_seconds - self.elapsed_seconds).max(0.0)
    }

    pub fn is_finished(self) -> bool {
        self.elapsed_seconds >= self.duration_seconds
    }

    pub fn tick(&mut self, delta_seconds: f32) {
        // Clamping keeps `elapsed_secs` meaningful for UI that reads it after
        // the cooldown has expired, matching Bevy's `TimerMode::Once`.
        self.elapsed_seconds = (self.elapsed_seconds + delta_seconds).min(self.duration_seconds);
    }
}

/// Cooldown timers for spells that have been cast.
///
/// This component tracks remaining cooldown time for each spell cast by an entity.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default)]
pub struct SpellCooldowns {
    /// Map of spell ID to remaining cooldown timer.
    pub timers: HashMap<SpellId, Cooldown>,
}

impl SpellCooldowns {
    /// Time elapsed on a spell's cooldown, or `None` if it is not on one.
    ///
    /// Note this returns *elapsed*, not remaining, despite the name — preserved
    /// as-is from the Bevy version because the cooldown UI is calibrated
    /// against it. See [`Cooldown::remaining_secs`] for the other one.
    pub fn get_remaining(&self, id: &SpellId) -> Option<f32> {
        self.timers.get(id).map(|timer| timer.elapsed_secs())
    }

    /// Check if a spell is currently on cooldown.
    pub fn is_on_cooldown(&self, id: &SpellId) -> bool {
        self.timers.get(id).is_some_and(|timer| !timer.is_finished())
    }

    /// Start a cooldown for a spell.
    pub fn start_cooldown(&mut self, id: SpellId, duration_seconds: f32) {
        self.timers.insert(id, Cooldown::new(duration_seconds));
    }

    /// Tick all cooldown timers.
    pub fn tick(&mut self, delta_seconds: f32) {
        for timer in self.timers.values_mut() {
            timer.tick(delta_seconds);
        }
    }

    /// Clean up finished cooldown timers to free memory.
    pub fn cleanup_finished(&mut self) {
        self.timers.retain(|_, timer| !timer.is_finished());
    }
}

/// Horizontal displacement threshold (in units) beyond which a cast-time
/// or an `InterruptOnMove` channeling spell is cancelled. Tunable.
pub const MOVEMENT_INTERRUPT_EPSILON: f32 = 0.05;

/// Server-authoritative state of a spell being cast (CastTime) or
/// channeled. The system [`crate::plugins::spells::systems::advance_cast_progress`]
/// ticks it every frame and decides when to trigger the effect.
///
/// Only one instance per caster: starting a new spell while this
/// component exists cancels the previous one.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug)]
pub struct CastProgress {
    pub spell_id: SpellId,
    pub kind: CastKind,
    pub elapsed_seconds: f32,
    /// For `CastTime`: required time before firing. For `Channeling` it is
    /// ignored (open-ended).
    pub required_seconds: f32,
    /// Movement interrupt policy, copied from `SpellConfig` at
    /// spawn time to avoid looking it up every tick.
    pub channel_movement: ChannelMovementPolicy,
    /// Snapshot of caster position at last tick, to detect
    /// movement that should interrupt the cast.
    pub last_position: Vec3,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<EntityId>,
    /// Active movement command when the cast started. Used to ignore
    /// previous movement and interrupt only on a new click/input.
    pub movement_input_at_start: Option<Vec3>,
    /// Accumulator for channeling tick interval. When it exceeds
    /// `tick_interval_seconds`, the spell is re-executed.
    pub channel_tick_accumulator_seconds: f32,
    pub tick_interval_seconds: f32,
}
