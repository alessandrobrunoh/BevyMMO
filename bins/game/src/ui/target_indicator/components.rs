//! Componenti per il target indicator (anello rosso sotto il target).

use bevy::prelude::*;

/// Marker per l'anello di selezione del target.
#[derive(Component)]
pub struct TargetSelectionRing;

/// Componente che traccia quale target l'anello sta seguendo.
#[derive(Component)]
pub struct TargetRingTarget {
    /// L'entity del target che l'anello sta seguendo.
    pub entity: Entity,
}
