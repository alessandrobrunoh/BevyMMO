use bevy::ecs::entity::MapEntities;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::entity::components::{EntityKind, EntityState, GameEntity, SpawnPoint};
use crate::plugins::spells::{HotbarSlot, SpellHotbar};
use crate::stats::components::{CombatStats, MovementStats, VitalStats};

// Channels
pub struct Channel1;

/// Reliable client -> server channel used for join messages (e.g. `JoinRequest`).
pub struct Channel2;

// Components
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(pub PeerId);

/// Generic position of a game entity, replicated via lightyear.
/// No longer specific to Player: any entity (Player, Enemy, NPC, ...)
/// can use it to have a replicated position in space.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct Position(pub Vec3);

impl Ease for Position {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            Position(Vec3::lerp(start.0, end.0, t))
        })
    }
}

/// Horizontal direction the entity is facing.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct LookDirection(pub Vec3);

impl Default for LookDirection {
    fn default() -> Self {
        Self(Vec3::Z)
    }
}

/// Generic color of a game entity, replicated via lightyear.
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct EntityColor(pub bevy::color::Color);

/// Stable gameplay identifier assigned by the server to replicated entities.
///
/// Unlike `Entity`, this value can be sent from the client to the
/// server to refer to the same selected target.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct NetworkEntityId(pub u64);

/// Replicated visual marker to distinguish spell projectiles from
/// gameplay entities rendered with generic meshes.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProjectileVisual {
    pub spell_id: String,
}

// Input commands
/// Point-and-click command sent from client to server.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Reflect)]
pub enum Inputs {
    MoveTo(Vec3),
    Stop,
}

impl Default for Inputs {
    fn default() -> Self {
        Self::Stop
    }
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

        app.component::<GameEntity>().replicate();

        app.component::<SpawnPoint>().replicate();

        app.component::<EntityKind>().replicate();

        app.component::<crate::plugins::entity::boss::components::Boss>()
            .replicate();
        app.component::<crate::plugins::entity::boss::components::BossPhase>()
            .replicate();
        app.component::<crate::plugins::entity::boss::components::BossArena>()
            .replicate();

        app.component::<crate::plugins::entity::components::PlayerName>()
            .replicate();

        app.component::<crate::plugins::crowd_control::CrowdControlState>()
            .replicate()
            .predict();
    }
}
