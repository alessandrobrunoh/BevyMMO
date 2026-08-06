//! Componenti condivise da tutte le entità di gioco.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Marker per qualsiasi entità di gioco (Player, Enemy, NPC, ...).
///
/// Il nome evita ambiguità con `bevy::ecs::entity::Entity`, che identifica
/// un'istanza ECS, non una categoria di gameplay.
#[derive(Component, Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GameEntity;

/// Stato comportamentale condiviso e replicato di un'entità di gioco.
///
/// Le transizioni che cambiano il gameplay devono essere decise dal server.
/// `Dead` è terminale finché un sistema esplicito di respawn non assegna un
/// nuovo stato.
#[derive(
    Component, Debug, Default, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq,
)]
#[reflect(Component)]
pub enum EntityState {
    #[default]
    Idle,
    Moving,
    Dead,
}

impl EntityState {
    pub fn is_dead(self) -> bool {
        self == Self::Dead
    }
}

/// Nome del player (solo UI e logging).
#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct PlayerName(pub String);

/// Tipo di entità di gioco per determinare alleanze e comportamento di targeting.
///
/// I valori influenzano UI (healthbar color, target frame), regole di targeting
/// e interazioni future. Il tipo viene replicato dal server e i client lo usano
/// per visual feedback, non per logica di gameplay autorevole.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq)]
#[reflect(Component)]
pub enum EntityKind {
    /// Player del client locale.
    Player,
    /// NPC alleati (es. mercanti, quest giver).
    Friendly,
    /// Creature neutrali che non attaccano per prime.
    Neutral,
    /// Nemici ostili (enemy, boss, creature aggressive).
    Hostile,
}
