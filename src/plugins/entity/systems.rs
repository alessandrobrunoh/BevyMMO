//! Sistemi condivisi da tutte le entità di gioco.
//!
//! Per ora: cleanup delle entità morte. Aggiungi qui i sistemi generici
//! che operano su `With<GameEntity>`.

use bevy::prelude::*;

use crate::stats::components::VitalStats;

/// Rimuove le entità con `VitalStats` esaurita.
pub fn despawn_dead_entities(
    mut commands: Commands,
    query: Query<(Entity, &VitalStats), Changed<VitalStats>>,
) {
    for (entity, vital) in query.iter() {
        if vital.is_dead() {
            commands.entity(entity).despawn();
        }
    }
}

// Usa `With<GameEntity>` nelle query che devono filtrare tutte le entità di gioco.
