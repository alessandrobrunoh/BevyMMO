use bevy::ecs::entity::MapEntities;
use bevy::prelude::*;
use lightyear::prelude::*;
/// Interpolation curve lightyear needs to smooth replicated positions.
///
/// Stays here rather than with the component: it exists only to satisfy
/// `add_linear_interpolation()`, and goes when lightyear does.
impl Ease for Position {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            Position(Vec3::lerp(start.0, end.0, t))
        })
    }
}

pub use crate::world_components::{
    EntityColor, LookDirection, NetworkEntityId, Position, ProjectileVisual,
};
use serde::{Deserialize, Serialize};

use crate::abilities::AbilitySlot;
use crate::spells::HotbarSlot;

// Components
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(pub PeerId);

// Input commands
/// Point-and-click command sent from client to server.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Reflect, Default)]
pub enum Inputs {
    MoveTo(Vec3),
    #[default]
    Stop,
}

impl MapEntities for Inputs {
    fn map_entities<M: EntityMapper>(&mut self, _entity_mapper: &mut M) {}
}

// Messages
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerMessage(pub usize);

/// Join request sent from client to server right after `Connected`.
/// The server validates `player_name` before spawning the player.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JoinRequest {
    pub player_name: String,
}

/// Client -> server command to request a spell cast.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SpellCastCommand {
    pub spell_id: String,
    pub target_position: Option<Vec3>,
    pub target_id: Option<u64>,
}

/// Client -> server command to release a channeling spell or
/// interrupt a CastTime spell. The client sends it on `just_released`
/// of the currently channeling spell key, or on re-press of the same
/// spell key (D2c: re-press = interrupt).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SpellCastRelease {
    pub spell_id: String,
}

/// Periodic snapshot sent from server to all clients to replicate the
/// state of a spell being cast or channeled. Used by the client to
/// position and fill the world-space cast bar above the caster.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bevy::prelude::Message)]
pub struct SpellCastProgress {
    /// Caster's `NetworkEntityId`, stable between server and client.
    pub caster_network_id: u64,
    pub spell_id: String,
    /// 0 = CastTime, 1 = Channeling.
    pub kind: u8,
    pub elapsed_seconds: f32,
    /// For CastTime: total wind-up duration. For Channeling: 0.0 (open-ended).
    pub required_seconds: f32,
}

/// Server -> client notification that a casting/channeling spell has
/// ended (completed or interrupted). The client removes the bar.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bevy::prelude::Message)]
pub struct SpellCastEnded {
    pub caster_network_id: u64,
    pub spell_id: String,
    /// `true` = cast completed normally, `false` = interrupted/cancelled.
    pub completed: bool,
}

/// Client -> server command to request respawn of the local player.
///
/// The server resolves the player from the sender peer and, if in `Dead` state,
/// brings them back to the spawn point with regenerated stats.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RespawnRequest;

/// Server -> client message to replicate a spell visual effect.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bevy::prelude::Message)]
pub struct SpellVisualEffect {
    pub spell_id: String,
    pub start: Vec3,
    pub end: Vec3,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateHotbarSlotRequest {
    pub slot: HotbarSlot,
    pub spell_id: Option<String>,
}

/// Client -> server command to cast the equipped weapon's Eidolon gesture at
/// `slot`. Unlike [`SpellCastCommand`], it carries no spell id: the server
/// resolves gesture + Incisione from the caster's equipped weapon and
/// `KnownGlyphs`. Supports Instant, CastTime, and Channeling via the unified
/// server pipeline (same cast bar / release flow as spell casts).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EidolonCastCommand {
    pub slot: AbilitySlot,
    pub target_position: Option<Vec3>,
    pub target_id: Option<u64>,
}
