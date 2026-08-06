//! Server-authoritative system for homing projectiles.
//!
//! Parallel module to [`crate::plugins::spells::aoe`]: manages the lifecycle of
//! any entity possessing an [`HomingProjectile`] component. The
//! effect payload (damage, hit_radius, speed) is carried by the
//! component itself, so the system is entirely agnostic to the
//! spell that spawned the projectile (today only `Fireball`, tomorrow
//! potentially other homing spells).

use bevy::prelude::*;

use crate::network::protocol::Position;
use crate::stats::components::VitalStats;
use crate::stats::events::DamageEvent;

/// Component marker for a homing projectile: pursues `target` at `speed`,
/// applies `damage` when entering `hit_radius`, then despawns.
#[derive(Component, Debug)]
pub struct HomingProjectile {
    pub target: Entity,
    pub speed: f32,
    pub damage: f32,
    pub hit_radius: f32,
}

/// Server-authoritative system: moves homing projectiles towards target
/// and applies damage on impact.
///
/// The two queries are made disjoint via `Without<HomingProjectile>`: targets
/// cannot be projectiles themselves, avoiding B0001 conflicts.
pub fn update_homing_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Position, &HomingProjectile)>,
    targets: Query<(&Position, &VitalStats), Without<HomingProjectile>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (proj_entity, mut proj_pos, proj) in projectiles.iter_mut() {
        // If target no longer exists, lacks Position/VitalStats or is dead, despawn.
        let Ok((target_pos, target_vital)) = targets.get(proj.target) else {
            commands.entity(proj_entity).despawn();
            continue;
        };
        if target_vital.is_dead() {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let direction = target_pos.0 - proj_pos.0;
        let distance = direction.length();

        // Hit: close enough to strike
        if distance <= proj.hit_radius {
            damage_events.write(DamageEvent {
                target: proj.target,
                source: None,
                amount: proj.damage,
            });
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Move towards target. `speed` is expressed in units/second.
        let step = (proj.speed * time.delta_secs()).min(distance);
        proj_pos.0 += direction / distance * step;
    }
}

