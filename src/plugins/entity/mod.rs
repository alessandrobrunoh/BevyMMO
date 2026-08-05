//! Entity plugin — container per tutte le entità di gioco (Player, Enemy, ...).
//!
//! Ogni entità concreta vive in un proprio sotto-modulo e registra il proprio
//! `EntityPlugin` figlio. Il plugin "padre" `EntityPlugin` raccoglie i figli
//! ed espone i tipi condivisi (`Entity` marker, `Health`, ...).

pub mod components;
pub mod definition;
pub mod spawn;
pub mod systems;

pub mod enemy;
pub mod player;

use bevy::prelude::*;

/// Plugin padre: registra tutti i plugin delle entità concrete.
pub struct EntityPlugin;

impl Plugin for EntityPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(enemy::EnemyPlugin);
    }
}
