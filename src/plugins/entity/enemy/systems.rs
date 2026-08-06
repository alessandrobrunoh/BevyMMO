//! Sistemi specifici dell'Enemy.
//!
//! AI e attacchi sono server-authoritative: il client riceve solo la posizione
//! e gli effetti replicati sulle entità, come la salute del Player.

use bevy::prelude::*;

use super::components::{AggroRange, Enemy};
use crate::network::protocol::Position;
use crate::plugins::entity::player::components::Player;
use crate::plugins::spells::{SpellCastRequest, SpellId};
use crate::stats::components::MovementStats;

/// AI: insegue il player più vicino se dentro `AggroRange`.
/// Server-only (registrato con `network::mode::has_server`).
pub fn enemy_chase(
    mut enemies: Query<(&mut Position, &AggroRange, &MovementStats), With<Enemy>>,
    players: Query<&Position, (With<Player>, Without<Enemy>)>,
) {
    if players.is_empty() {
        return;
    }

    for (mut enemy_pos, aggro, stats) in enemies.iter_mut() {
        // Trova il player più vicino
        let nearest = players.iter().min_by(|a, b| {
            a.0.distance_squared(enemy_pos.0)
                .partial_cmp(&b.0.distance_squared(enemy_pos.0))
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        if let Some(target) = nearest {
            let distance = enemy_pos.0.distance(target.0);
            if distance <= aggro.0 && distance > 0.001 {
                let direction = (target.0 - enemy_pos.0).normalize();
                enemy_pos.0 += direction * stats.speed;
            }
        }
    }
}

/// AI: lancia automaticamente l'attacco quando un player è nel raggio.
/// Server-only (registrato con `network::mode::has_server`).
pub fn enemy_auto_cast_attack(
    enemies: Query<(Entity, &Position, &AggroRange), With<Enemy>>,
    players: Query<&Position, With<Player>>,
    mut spell_cast_requests: MessageWriter<SpellCastRequest>,
) {
    for (enemy_entity, enemy_position, aggro) in enemies.iter() {
        // Trova il player più vicino
        let nearest = players.iter().min_by(|a, b| {
            a.0.distance_squared(enemy_position.0)
                .partial_cmp(&b.0.distance_squared(enemy_position.0))
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        if let Some(target) = nearest {
            let distance = enemy_position.0.distance(target.0);
            if distance <= aggro.0 {
                spell_cast_requests.write(SpellCastRequest {
                    caster: enemy_entity,
                    spell_id: SpellId::new("attack"),
                    target_position: None,
                    target_entity: None,
                });
            }
        }
    }
}
