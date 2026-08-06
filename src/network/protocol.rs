use bevy::ecs::entity::MapEntities;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::entity::components::{EntityKind, EntityState, GameEntity, SpawnPoint};
use crate::plugins::spells::{HotbarSlot, SpellHotbar};
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
pub struct EntityColor(pub bevy::color::Color);

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

/// Comando client -> server per rilasciare una spell channeling o
/// interrompere una spell CastTime. Il client lo invia su `just_released`
/// della spell key attualmente channeling, oppure su re-press della stessa
/// spell key (D2c: re-press = interrompi).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SpellCastRelease {
    pub spell_id: String,
}

/// Snapshot periodico inviato dal server a tutti i client per replicare lo
/// stato di una spell in fase di cast o channeling. Usato dal client per
/// posizionare e riempire la barra di cast world-space sopra il caster.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bevy::prelude::Message)]
pub struct SpellCastProgress {
    /// `NetworkEntityId` del caster, stabile tra server e client.
    pub caster_network_id: u64,
    pub spell_id: String,
    /// 0 = CastTime, 1 = Channeling.
    pub kind: u8,
    pub elapsed_seconds: f32,
    /// Per CastTime: durata totale del wind-up. Per Channeling: 0.0 (aperto).
    pub required_seconds: f32,
}

/// Notifica server -> client che una spell in fase di cast/channeling è
/// terminata (completata o interrotta). Il client rimuove la barra.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bevy::prelude::Message)]
pub struct SpellCastEnded {
    pub caster_network_id: u64,
    pub spell_id: String,
    /// `true` = cast completato normalmente, `false` = interrotto/cancellato.
    pub completed: bool,
}

/// Comando client -> server per richiedere il respawn del player locale.
///
/// Il server risolve il player dal peer mittente e, se è in stato `Dead`,
/// lo riporta allo spawn point con statistiche rigenerate.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RespawnRequest;

/// Messaggio server -> client per replicare un effetto visivo spell.
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

        app.component::<SpellHotbar>().replicate().predict();

        app.component::<GameEntity>().replicate();

        app.component::<SpawnPoint>().replicate();

        app.component::<EntityKind>().replicate();

        app.component::<crate::plugins::entity::components::PlayerName>()
            .replicate();

        app.component::<crate::plugins::crowd_control::CrowdControlState>()
            .replicate()
            .predict();
    }
}
