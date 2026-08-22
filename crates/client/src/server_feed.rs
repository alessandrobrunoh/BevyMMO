//! Server-to-client news that is not entity state.
//!
//! Everything the database mirrors — positions, stats, inventories — becomes a
//! *component* on a mirrored entity, and the presentation reads it with an
//! ordinary query. These do not fit that shape:
//!
//! - a rejected reducer is an answer to something the player just did, not a
//!   fact about the world;
//! - a line of chat or a server announcement belongs to a log, not to an entity;
//! - a cooldown belongs to the *hotbar*, which is UI state keyed by ability id
//!   rather than by entity;
//! - a one-shot world label (gather yield, later damage numbers) is a cue, not
//!   a replicated component.
//!
//! They are Bevy messages instead, written by the SpacetimeDB bridge and read by
//! the presentation. The types live here, in the crate both sides already
//! depend on, rather than in `network::protocol` — that module is the outgoing
//! lightyear wire format, and adding to it would tie new code to something on
//! its way out.

use bevy::prelude::{Color, Message, Vec3};
use serde::{Deserialize, Serialize};

const DEFAULT_WORLD_TEXT_LIFETIME_SECONDS: f32 = 1.5;
const DEFAULT_WORLD_TEXT_RISE_SPEED: f32 = 1.25;
const DEFAULT_WORLD_TEXT_FONT_SIZE: f32 = 18.0;

/// How loudly a [`ServerNotice`] wants to be shown.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeSeverity {
    /// Something happened, and the player may like to know: a respawn, a
    /// greeting, a broadcast.
    Info,
    /// The server refused what the player asked for. This is the case that had
    /// no channel at all before: the module wrote careful messages — "inventory
    /// is full", "target is out of range", "that name is taken" — and every one
    /// of them was discarded by a fire-and-forget send.
    Error,
}

/// One line of text for the player.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Message)]
pub struct ServerNotice {
    pub text: String,
    pub severity: NoticeSeverity,
}

impl ServerNotice {
    /// A neutral announcement.
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            severity: NoticeSeverity::Info,
        }
    }

    /// A refusal, phrased as the server phrased it.
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            severity: NoticeSeverity::Error,
        }
    }

    /// Whether this notice reports a refusal.
    pub fn is_error(&self) -> bool {
        self.severity == NoticeSeverity::Error
    }
}

/// A line delivered by the global server chat or a system message.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct ChatLine {
    pub text: String,
}

/// A one-shot label in the world, not attached to any entity.
///
/// Presentation turns this into a floating UI node. The client writes it so
/// `bevymmo_client` does not depend on presentation types.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct WorldTextCue {
    pub world_position: Vec3,
    pub text: String,
    pub color: Color,
    pub lifetime_seconds: f32,
    pub rise_speed: f32,
    pub font_size: f32,
}

impl WorldTextCue {
    pub fn new(world_position: Vec3, text: impl Into<String>) -> Self {
        Self {
            world_position,
            text: text.into(),
            color: Color::WHITE,
            lifetime_seconds: DEFAULT_WORLD_TEXT_LIFETIME_SECONDS,
            rise_speed: DEFAULT_WORLD_TEXT_RISE_SPEED,
            font_size: DEFAULT_WORLD_TEXT_FONT_SIZE,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_lifetime(mut self, seconds: f32) -> Self {
        self.lifetime_seconds = seconds;
        self
    }

    pub fn with_rise_speed(mut self, speed: f32) -> Self {
        self.rise_speed = speed;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
}

/// The authoritative state of one cooldown on one entity.
///
/// The HUD used to start its own timers the moment a key was pressed, which is
/// right up until the server disagrees — a cast that was refused still greyed
/// the key out, and a cooldown shortened by a buff stayed grey for the full
/// original duration. This carries what the `cooldown` table actually says.
///
/// `remaining_seconds` of zero means the cooldown ended; the row is gone and
/// the key is ready.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Message)]
pub struct SpellCooldownState {
    /// The entity the cooldown belongs to, as `game_entity.entity_id`.
    pub entity_id: u64,
    /// Spell id or ability id — the module keeps them in one namespace, because
    /// a cooldown is a cooldown regardless of what started it.
    pub ability_id: String,
    pub remaining_seconds: f32,
    pub duration_seconds: f32,
}

impl SpellCooldownState {
    /// Whether the ability is ready again.
    pub fn is_ready(&self) -> bool {
        self.remaining_seconds <= 0.0
    }
}
