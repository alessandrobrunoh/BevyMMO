//! Enemy-specific components.

use bevy::prelude::*;

/// Marker component for enemy (server-controlled AI).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Enemy;

/// Aggro radius: within this distance from the target, the enemy pursues it.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct AggroRange(pub f32);

impl Default for AggroRange {
    fn default() -> Self {
        Self(10.0)
    }
}

/// Respawn timer: attached to an `Enemy` when it enters `EntityState::Dead`.
/// The `enemy_respawn` system decrements it until expiry, after which
/// the enemy is revived at its `SpawnPoint`.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Respawning {
    pub remaining: f32,
}

/// Respawn duration of the enemy after death, in seconds.
pub const ENEMY_RESPAWN_SECONDS: f32 = 10.0;

