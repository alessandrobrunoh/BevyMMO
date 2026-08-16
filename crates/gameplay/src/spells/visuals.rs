//! Visual effects a spell asks the client to play.
//!
//! Emitted into `SpellCastContext::pending_visuals` and drained by whoever ran
//! the cast. Previously a lightyear message in `network::protocol`; it is plain
//! data, and the transport it travels over is not its concern.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// A one-shot visual: a bolt from `start` to `end`, a burst at a point, and so
/// on. `spell_id` tells the client which effect to play.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::message::Message))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SpellVisualEffect {
    pub spell_id: String,
    pub start: Vec3,
    pub end: Vec3,
}
