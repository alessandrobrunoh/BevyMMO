//! Spawn del projectile replicato per la spell Fireball.
//!
//! A differenza di RayOfLight/HealingCircle (il cui visual è un effetto effimero
//! client-side gestito tramite `SpellVisualEffect`), la Fireball crea una
//! entity server-authoritative replicata: position, colore, marker visual e
//! componente homing. Tutta la costruzione del bundle vive qui.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::{EntityColor, Position, ProjectileVisual};
use crate::plugins::spells::HomingProjectile;
use crate::plugins::spells::ProjectileSpawnRequest;

use lightyear::prelude::{NetworkTarget, Replicate};

/// Colore del proiettile Fireball (azzurro).
pub const PROJECTILE_COLOR: Color = Color::srgb(0.2, 0.8, 1.0);

/// Spawna l'entità projectile replicata per Fireball.
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
