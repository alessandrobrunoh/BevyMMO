use bevy::ecs::entity::MapEntities;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::entity::components::{EntityKind, EntityState, GameEntity};
use crate::stats::components::{CombatStats, MovementStats, VitalStats};

// Canali
pub struct Channel1;

/// Canale affidabile client -> server usato per i messaggi di join (es. `JoinRequest`).
pub struct Channel2;

// Componenti
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(pub PeerId);

/// Posizione generica di un'entità di gioco, replicata via lightyear.
/// Non è più specifica del Player: qualsiasi entità (Player, Enemy, NPC, ...)
/// può usarla per avere una posizione nello spazio replicata.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct Position(pub Vec3);

impl Ease for Position {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            Position(Vec3::lerp(start.0, end.0, t))
        })
    }
}

/// Direzione orizzontale in cui l'entità sta guardando.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct LookDirection(pub Vec3);

impl Default for LookDirection {
    fn default() -> Self {
        Self(Vec3::Z)
    }
}

/// Colore generico di un'entità di gioco, replicato via lightyear.
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct EntityColor(pub bevy::prelude::Color);

/// Identificatore gameplay stabile assegnato dal server alle entità replicate.
///
/// A differenza di `Entity`, questo valore può essere inviato dal client al
/// server per riferirsi allo stesso target selezionato.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct NetworkEntityId(pub u64);

/// Marker visuale replicato per distinguere i projectile spell dalle entità
/// gameplay renderizzate con mesh generica.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProjectileVisual {
    pub spell_id: String,
}

// Comandi di input
/// Comando punta-e-clicca inviato dal client al server.
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

// Messaggi
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerMessage(pub usize);

/// Richiesta di join inviata dal client al server subito dopo `Connected`.
/// Il server valida `player_name` prima di spawnare il player.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JoinRequest {
    pub player_name: String,
}

/// Comando client -> server per richiedere il cast di una spell.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SpellCastCommand {
    pub spell_id: String,
    pub target_position: Option<Vec3>,
    pub target_id: Option<u64>,
}

/// Messaggio server -> client per replicare un effetto visivo spell.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bevy::prelude::Message)]
pub struct SpellVisualEffect {
    pub spell_id: String,
    pub start: Vec3,
    pub end: Vec3,
}

// Protocol Plugin
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Canali
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

        // Messaggi
        app.register_message::<PlayerMessage>()
            .add_direction(NetworkDirection::ServerToClient);

        app.register_message::<JoinRequest>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<SpellCastCommand>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<SpellVisualEffect>()
            .add_direction(NetworkDirection::ServerToClient);

        // Comandi di input
        app.add_plugins(input::native::InputPlugin::<Inputs>::default());

        // Componenti
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

        app.component::<GameEntity>().replicate();

        app.component::<EntityKind>().replicate();

        app.component::<crate::plugins::entity::components::PlayerName>()
            .replicate();
    }
}
