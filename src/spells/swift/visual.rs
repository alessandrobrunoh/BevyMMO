//! Visual client-side per Swift: feedback minimale, il segnale principale è la
//! barra di channeling visibile sopra il caster.

use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;

/// Spawn placeholder (no-op): l'effetto visivo principale è la barra di cast.
pub fn spawn(
    _commands: &mut Commands,
    _meshes: &mut Assets<Mesh>,
    _materials: &mut Assets<StandardMaterial>,
    _effect: &SpellVisualEffect,
) {
}

/// Animate placeholder.
pub fn animate(
    _time: Res<Time>,
    _commands: Commands,
    _visuals: Query<Entity, With<SpellVisual>>,
) {
}
