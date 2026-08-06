//! Componenti specifiche dell'Enemy.

use bevy::prelude::*;

/// Marker component per enemy (AI controllata dal server).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Enemy;

/// Raggio di aggro: entro questa distanza dal target, l'enemy lo insegue.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct AggroRange(pub f32);

impl Default for AggroRange {
    fn default() -> Self {
        Self(10.0)
    }
}
