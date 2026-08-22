//! ECS components for ability cast runtime state.

use crate::EntityId;
use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::context::{CastKind, ChannelMovementPolicy};

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
    pub spell_id: String,
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
