//! Enemy-specific components.

// `#[reflect(Component)]` expands to a reference to this type.
#[cfg(feature = "bevy")]
use bevy_ecs::reflect::ReflectComponent;

/// Marker component for enemy (server-controlled AI).
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default, Clone, Copy)]
pub struct Enemy;

/// Aggro radius: within this distance from the target, the enemy pursues it.
#[cfg_attr(
    feature = "bevy",
    derive(bevy_ecs::component::Component, bevy_reflect::Reflect)
)]
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct AggroRange(pub f32);

impl Default for AggroRange {
    fn default() -> Self {
        Self(10.0)
    }
}

/// Respawn timer: attached to an `Enemy` when it enters `EntityState::Dead`.
/// The `enemy_respawn` system decrements it until expiry, after which
/// the enemy is revived at its `SpawnPoint`.
#[cfg_attr(
    feature = "bevy",
    derive(bevy_ecs::component::Component, bevy_reflect::Reflect)
)]
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct Respawning {
    pub remaining: f32,
}

/// Respawn duration of the enemy after death, in seconds.
pub const ENEMY_RESPAWN_SECONDS: f32 = 10.0;
