//! Sistemi condivisi da tutte le entità di gioco.
//!
//! Per ora: cleanup delle entità morte. Aggiungi qui i sistemi generici
//! che operano su `With<Entity>`.

use bevy::prelude::*;

use super::components::*;

/// Rimuove le entità con `Health` esaurita.
pub fn despawn_dead_entities(
    mut commands: Commands,
    query: Query<(Entity, &Health), Changed<Health>>,
) {
    for (entity, health) in query.iter() {
        if health.is_dead() {
            commands.entity(entity).despawn();
        }
    }
}

// Usa `With<GameEntity>` nelle query che devono filtrare tutte le entità di gioco.
