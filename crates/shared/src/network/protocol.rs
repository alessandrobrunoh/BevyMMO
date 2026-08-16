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

use crate::entity::components::{EntityKind, EntityState, GameEntity, SpawnPoint};
use crate::abilities::{AbilitySlot, KnownGlyphs};
use crate::items::components::{Equipment, Inventory};
use crate::items::events::{EquipItemCommand, MoveItemCommand, UnequipItemCommand};
use crate::spells::{HotbarSlot, SpellHotbar};
use crate::stats::components::{CombatStats, MovementStats, VitalStats};

// Channels
pub struct Channel1;

/// Reliable client -> server channel used for join messages (e.g. `JoinRequest`).
pub struct Channel2;

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

/// Reliable point-and-click movement command. This provides the authoritative
/// server target independently of the prediction input timeline.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MoveCommand {
    pub target: Vec3,
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

/// Client -> server command to inscribe (or clear) one slot of the
/// equipped weapon's Incisione. `essence`/`modifiers`/`ancient_word` are
/// glyph ids as strings (empty `modifiers` = no modifiers); the server
/// validates ownership (`KnownGlyphs`), tag compatibility, and total Runic
/// Capacity before applying — an invalid request is rejected wholesale.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateInscriptionRequest {
    pub slot: AbilitySlot,
    pub essence: Option<String>,
    pub modifiers: Vec<String>,
    pub ancient_word: Option<String>,
}

/// Picks which of the weapon's offered gestures is active on `slot`.
///
/// Primary/Secondary offer 1+ `BaseAbility` options each (Ultimate exactly
/// one), so the player chooses one per slot; the server rejects an
/// `ability_id` the equipped weapon doesn't offer for that slot. Changing the
/// gesture can invalidate the slot's Incisione (tags differ between gestures),
/// in which case the server clears that slot's glyphs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateAbilitySelectionRequest {
    pub slot: AbilitySlot,
    pub ability_id: String,
}

// Protocol Plugin
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Channels
        app.add_channel::<Channel1>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);

        app.add_channel::<Channel2>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);

        // Messages
        app.register_message::<PlayerMessage>()
            .add_direction(NetworkDirection::ServerToClient);

        app.register_message::<JoinRequest>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<MoveCommand>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<SpellCastCommand>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<SpellCastRelease>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<RespawnRequest>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<SpellVisualEffect>()
            .add_direction(NetworkDirection::ServerToClient);

        app.register_message::<SpellCastProgress>()
            .add_direction(NetworkDirection::ServerToClient);

        app.register_message::<SpellCastEnded>()
            .add_direction(NetworkDirection::ServerToClient);

        app.register_message::<UpdateHotbarSlotRequest>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<EquipItemCommand>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<UnequipItemCommand>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<MoveItemCommand>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<EidolonCastCommand>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<UpdateInscriptionRequest>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<UpdateAbilitySelectionRequest>()
            .add_direction(NetworkDirection::ClientToServer);

        // Input commands
        app.add_plugins(input::native::InputPlugin::<Inputs>::default());

        // Components
        app.component::<PlayerId>().replicate();

        app.component::<Position>()
            .replicate()
            .predict()
            .add_linear_interpolation();

        app.component::<EntityColor>().replicate();

        app.component::<NetworkEntityId>().replicate();

        app.component::<ProjectileVisual>().replicate();

        app.component::<LookDirection>().replicate().predict();

        app.component::<MovementStats>().replicate().predict();

        app.component::<CombatStats>().replicate().predict();

        app.component::<VitalStats>().replicate().predict();

        app.component::<EntityState>().replicate().predict();

        app.component::<SpellHotbar>().replicate().predict();

        app.component::<Inventory>().replicate().predict();

        app.component::<Equipment>().replicate().predict();

        // Read-only for the owning client: rendered by the inscription UI to
        // filter which Glifi are pickable, never written to locally (only
        // the server ever changes it, and nothing does yet — no
        // learn-a-glyph flow exists). `.predict()` would be wasted, plain
        // replication is enough.
        app.component::<KnownGlyphs>().replicate();

        app.component::<GameEntity>().replicate();

        app.component::<SpawnPoint>().replicate();

        app.component::<EntityKind>().replicate();

        app.component::<crate::entity::boss::components::Boss>()
            .replicate();
        app.component::<crate::entity::boss::components::BossPhase>()
            .replicate();
        app.component::<crate::entity::boss::components::BossArena>()
            .replicate();

        app.component::<crate::entity::components::PlayerName>()
            .replicate();

        app.component::<crate::crowd_control::CrowdControlState>()
            .replicate()
            .predict();
    }
}
