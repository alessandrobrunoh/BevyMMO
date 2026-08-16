//! Componenti per la sidebar NPC (pannello informativo su click NPC).

use bevy::prelude::*;

/// Marker per la sidebar NPC attualmente aperta.
///
/// Traccia l'entity target della sidebar corrente per poterla aggiornare o
/// rimuovere quando necessario.
#[derive(Component, Debug)]
pub struct NpcSidebar {
    /// L'entity del NPC che questa sidebar sta mostrando.
    pub target: Entity,
}
