//! Spawn of the replicated projectile for the Fireball spell.
//!
//! Unlike RayOfLight/HealingCircle (whose visual is an ephemeral
//! client-side effect managed via `SpellVisualEffect`), Fireball creates a
//! server-authoritative replicated entity: position, color, visual marker, and
//! homing component. All bundle construction lives here.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::{EntityColor, Position, ProjectileVisual};
use crate::plugins::spells::HomingProjectile;
use crate::plugins::spells::ProjectileSpawnRequest;

use lightyear::prelude::{NetworkTarget, Replicate};

/// Fireball projectile color (light blue).
pub const PROJECTILE_COLOR: Color = Color::srgb(0.2, 0.8, 1.0);

/// Spawns the replicated projectile entity for Fireball.
pub fn spawn(commands: &mut Commands, start: Vec3, request: ProjectileSpawnRequest) {
    commands.spawn((
        Position(start),
        EntityColor(PROJECTILE_COLOR),
        ProjectileVisual {
            spell_id: crate::spells::fireball::FireballSpell::ID.to_string(),
        },
        HomingProjectile {
            target: request.target,
            speed: request.speed,
            damage: request.damage,
            hit_radius: request.hit_radius,
        },
        Replicate::to_clients(NetworkTarget::All),
    ));
}
