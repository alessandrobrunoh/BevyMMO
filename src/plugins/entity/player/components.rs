//! Componenti specifiche del Player.

use bevy::prelude::*;

/// Marker component per il player. Va inserito sull'entità di gioco controllata
/// da un client. Le componenti di rete (`PlayerId`, `PlayerPosition`, `PlayerColor`)
/// restano in `crate::network::protocol` perché legate alla replicazione lightyear.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Player;
