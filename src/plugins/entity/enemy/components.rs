//! Componenti specifiche dell'Enemy.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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

/// Configurazione dell’attacco ad area dell’Enemy.
///
/// È replicata ai client per permettere al debug visuale di usare lo stesso
/// raggio del calcolo server-side.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct EnemyAttack {
    pub radius: f32,
    pub cooldown_seconds: f32,
}

impl Default for EnemyAttack {
    fn default() -> Self {
        Self {
            radius: 3.0,
            cooldown_seconds: 1.0,
        }
    }
}
