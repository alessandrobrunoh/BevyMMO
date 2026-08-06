//! Dummy entity — bersaglio statico per test danni e UI.
//!
//! Il Dummy è un'entità ferma con HP enormi, utile per testare sistema
//! di danni, UI targeting, healthbar colorate e spell senza il player che
//! si muova o l'enemy che reagisca.

pub mod components;
pub mod spawn;

use bevy::prelude::*;

/// Plugin Dummy: registra il bundle e l'EntityDefinition.
pub struct DummyPlugin;

impl Plugin for DummyPlugin {
    fn build(&self, _app: &mut App) {
        // Nessun sistema specifico: il Dummy è solo un'entità statica
    }
}
