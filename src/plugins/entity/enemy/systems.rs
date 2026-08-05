//! Sistemi specifici dell'Enemy.
//!
//! AI e attacchi sono server-authoritative: il client riceve solo la posizione
//! e gli effetti replicati sulle entità, come la salute del Player.

use bevy::prelude::*;

use super::components::{AggroRange, Enemy, EnemyAttack};
use crate::network::protocol::Position;
use crate::plugins::entity::components::{Health, Stats};
use crate::plugins::entity::player::components::Player;

/// Cooldown runtime dell'attacco: resta server-only e non viene replicato.
#[derive(Component)]
pub struct EnemyAttackCooldown(pub Timer);

/// AI: insegue il player più vicino se dentro `AggroRange`.
/// Server-only (registrato con `network::mode::has_server`).
pub fn enemy_chase(
    mut enemies: Query<(&mut Position, &AggroRange, &Stats), With<Enemy>>,
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

/// Inserisce il timer sugli Enemy già presenti, evitando stato di cooldown
/// replicato o persistito.
pub fn initialize_attack_cooldowns(
    mut commands: Commands,
    enemies: Query<(Entity, &EnemyAttack), Without<EnemyAttackCooldown>>,
) {
    for (entity, attack) in enemies.iter() {
        commands.entity(entity).insert(EnemyAttackCooldown(Timer::from_seconds(
            attack.cooldown_seconds.max(0.01),
            TimerMode::Repeating,
        )));
    }
}

/// Infligge danno a tutti i Player nel raggio quando il cooldown termina.
pub fn enemy_area_attack(
    time: Res<Time<Fixed>>,
    mut enemies: Query<
        (&Position, &Stats, &EnemyAttack, &mut EnemyAttackCooldown),
        With<Enemy>,
    >,
    mut players: Query<
        (&Position, &mut Health, &Stats),
        (With<Player>, Without<Enemy>),
    >,
) {
    for (enemy_position, enemy_stats, attack, mut cooldown) in enemies.iter_mut() {
        cooldown.0.tick(time.delta());
        if !cooldown.0.just_finished() {
            continue;
        }

        for (player_position, mut health, player_stats) in players.iter_mut() {
            if !is_in_attack_radius(enemy_position.0, player_position.0, attack.radius) {
                continue;
            }

            let damage = damage_after_armor(enemy_stats.damage, player_stats);
            health.current = (health.current - damage).max(0.0);
        }
    }
}

fn is_in_attack_radius(enemy_position: Vec3, player_position: Vec3, radius: f32) -> bool {
    enemy_position.distance_squared(player_position) <= radius.max(0.0).powi(2)
}

fn damage_after_armor(raw_damage: f32, target_stats: &Stats) -> f32 {
    (raw_damage * (1.0 - target_stats.damage_reduction())).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_damage_respects_target_armor() {
        let target = Stats::with_combat_values(0.15, 10.0, 100.0, 100.0, 5.0, 100.0);

        assert_eq!(damage_after_armor(10.0, &target), 5.0);
    }

    #[test]
    fn area_attack_includes_the_radius_boundary_but_not_points_outside() {
        assert!(is_in_attack_radius(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0), 3.0));
        assert!(!is_in_attack_radius(Vec3::ZERO, Vec3::new(3.01, 0.0, 0.0), 3.0));
    }

    #[test]
    fn area_damage_never_heals_or_goes_below_zero() {
        let target = Stats::with_combat_values(0.15, 10.0, 100.0, 100.0, 5.0, 0.0);

        assert_eq!(damage_after_armor(-10.0, &target), 0.0);
    }
}
