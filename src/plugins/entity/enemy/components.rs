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

/// Timer di respawn: attachato a un `Enemy` quando entra in `EntityState::Dead`.
/// Il sistema `enemy_respawn` lo decrementa finché non scade, dopodiché
/// l'enemy torna in vita allo `SpawnPoint`.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Respawning {
    pub remaining: f32,
}

/// Tempo di respawn del nemico dopo la morte, in secondi.
pub const ENEMY_RESPAWN_SECONDS: f32 = 10.0;
