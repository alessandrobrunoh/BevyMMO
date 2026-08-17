//! Componenti per il target frame (UI panel con info sul target selezionato).

use bevy::prelude::*;

/// Marker per il pannello del target frame.
#[derive(Component)]
pub struct TargetFrame;

/// Componente che traccia quale target il frame sta seguendo.
#[derive(Component)]
pub struct TargetFrameTarget {
    /// L'entity del target che il frame sta seguendo.
    pub entity: Entity,
}

/// Riferimenti diretti alle parti del target frame per aggiornamenti efficienti.
#[derive(Component)]
pub struct TargetFrameParts {
    pub name_text: Entity,
    pub hp_text: Entity,
    pub kind_text: Entity,
    pub hp_fill: Entity,
    pub last_name: String,
    pub last_hp_text: String,
    pub last_kind_text: String,
    pub last_hp_pct: f32,
}

impl TargetFrameParts {
    pub fn new(name_text: Entity, hp_text: Entity, kind_text: Entity, hp_fill: Entity) -> Self {
        Self {
            name_text,
            hp_text,
            kind_text,
            hp_fill,
            last_name: String::new(),
            last_hp_text: String::new(),
            last_kind_text: String::new(),
            last_hp_pct: -1.0,
        }
    }
}
