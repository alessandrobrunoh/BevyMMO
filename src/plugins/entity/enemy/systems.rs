//! Sistemi specifici dell'Enemy.
//!
//! AI e attacchi sono server-authoritative: il client riceve solo la posizione
//! e gli effetti replicati sulle entità, come la salute del Player.

use bevy::prelude::*;

use super::components::{AggroRange, Enemy, Respawning, ENEMY_RESPAWN_SECONDS};
use crate::network::protocol::Position;
use crate::plugins::entity::components::{EntityState, SpawnPoint};
use crate::plugins::entity::events::RespawnedEvent;
use crate::plugins::entity::player::components::Player;
use crate::plugins::spells::{SpellCastRequest, SpellId};
use crate::stats::components::{MovementStats, VitalStats};

/// AI: insegue il player più vicino se dentro `AggroRange`.
/// Server-only. Gli enemy in stato `Dead` sono saltati (non si muovono).
pub fn enemy_chase(
    mut enemies: Query<
        (&mut Position, &AggroRange, &MovementStats, &EntityState),
        (With<Enemy>, Without<Respawning>),
    >,
    players: Query<&Position, (With<Player>, Without<Enemy>)>,
) {
    if players.is_empty() {
        return;
    }

    for (mut enemy_pos, aggro, stats, state) in enemies.iter_mut() {
        if state.is_dead() {
            continue;
        }
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
/// Server-only. Gli enemy in stato `Dead` sono saltati.
pub fn enemy_auto_cast_attack(
    enemies: Query<
        (Entity, &Position, &AggroRange, &EntityState),
        (With<Enemy>, Without<Respawning>),
    >,
    players: Query<&Position, With<Player>>,
    mut spell_cast_requests: MessageWriter<SpellCastRequest>,
) {
    for (enemy_entity, enemy_position, aggro, state) in enemies.iter() {
        if state.is_dead() {
            continue;
        }
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

/// Attacca `Respawning` agli enemy appena entrati in `Dead`.
///
/// Filtra con `Without<Respawning>` così è idempotente: un enemy già in
/// countdown non riceve un secondo timer.
pub fn schedule_enemy_respawn(
    mut commands: Commands,
    enemies: Query<(Entity, &EntityState), (With<Enemy>, Without<Respawning>)>,
) {
    for (entity, state) in enemies.iter() {
        if state.is_dead() {
            commands.entity(entity).insert(Respawning {
                remaining: ENEMY_RESPAWN_SECONDS,
            });
        }
    }
}

/// Decrementa il timer di respawn e, quando scade, riporta l'enemy in vita
/// allo `SpawnPoint` con HP/mana rigenerati e `EntityState::Idle`.
pub fn enemy_respawn(
    time: Res<Time>,
    mut commands: Commands,
    mut respawned: MessageWriter<RespawnedEvent>,
    mut query: Query<(
        Entity,
        &mut Respawning,
        &mut Position,
        &mut VitalStats,
        &mut EntityState,
        &SpawnPoint,
    )>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, mut respawning, mut position, mut vital, mut state, spawn) in query.iter_mut() {
        respawning.remaining -= delta;
        if respawning.remaining > 0.0 {
            continue;
        }
        position.0 = spawn.0;
        vital.current_health = vital.max_health;
        vital.clamp_health();
        *state = EntityState::Idle;
        commands.entity(entity).remove::<Respawning>();
        respawned.write(RespawnedEvent { entity });
        info!("Enemy {:?} respawned at {:?}", entity, spawn.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::protocol::Position;
    use crate::plugins::entity::components::SpawnPoint;
    use crate::stats::components::{CombatStats, MovementStats};

    fn make_dead_enemy(app: &mut App, spawn: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                Enemy,
                EntityState::Dead,
                Position(spawn),
                SpawnPoint(spawn),
                Respawning { remaining: 0.5 },
                VitalStats {
                    current_health: 0.0,
                    max_health: 50.0,
                    max_mana: 40.0,
                    mana_regeneration: 2.0,
                },
            ))
            .id()
    }

    #[test]
    fn enemy_respawn_resets_state_position_and_health_when_timer_expires() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_message::<RespawnedEvent>();
        app.add_systems(Update, enemy_respawn);

        let spawn_point = Vec3::new(5.0, 0.0, 5.0);
        let entity = make_dead_enemy(&mut app, spawn_point);

        // Avanza il tempo oltre il timer rimanente (0.5s).
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0));
        app.update();

        let entity_ref = app.world().entity(entity);
        assert_eq!(
            entity_ref.get::<EntityState>().copied().unwrap(),
            EntityState::Idle
        );
        assert_eq!(entity_ref.get::<Position>().unwrap().0, spawn_point);
        let vital = entity_ref.get::<VitalStats>().unwrap();
        assert_eq!(vital.current_health, vital.max_health);
        assert!(!entity_ref.contains::<Respawning>());
    }

    #[test]
    fn enemy_respawn_keeps_entity_dead_when_timer_has_not_expired() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_message::<RespawnedEvent>();
        app.add_systems(Update, enemy_respawn);

        let spawn_point = Vec3::new(2.0, 0.0, 1.0);
        let entity = make_dead_enemy(&mut app, spawn_point);

        // Avanza il tempo di meno del rimanente.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        app.update();

        let entity_ref = app.world().entity(entity);
        assert_eq!(
            entity_ref.get::<EntityState>().copied().unwrap(),
            EntityState::Dead
        );
        assert!(entity_ref.contains::<Respawning>());
    }

    #[test]
    fn schedule_enemy_respawn_only_marks_dead_enemies_without_respawning() {
        let mut app = App::new();
        app.add_systems(Update, schedule_enemy_respawn);

        let dead = app
            .world_mut()
            .spawn((
                Enemy,
                EntityState::Dead,
                Position(Vec3::ZERO),
                SpawnPoint(Vec3::ZERO),
            ))
            .id();
        let alive = app
            .world_mut()
            .spawn((
                Enemy,
                EntityState::Idle,
                Position(Vec3::ZERO),
                SpawnPoint(Vec3::ZERO),
            ))
            .id();
        let already_respawning = app
            .world_mut()
            .spawn((
                Enemy,
                EntityState::Dead,
                Position(Vec3::ZERO),
                SpawnPoint(Vec3::ZERO),
                Respawning { remaining: 3.0 },
            ))
            .id();

        app.update();

        assert!(app.world().entity(dead).contains::<Respawning>());
        assert!(!app.world().entity(alive).contains::<Respawning>());
        assert_eq!(
            app.world()
                .entity(already_respawning)
                .get::<Respawning>()
                .unwrap()
                .remaining,
            3.0,
            "existing timer should not be overwritten"
        );
    }

    #[test]
    fn dead_enemies_do_not_chase_or_attack() {
        let mut app = App::new();
        app.add_systems(Update, enemy_chase);

        let enemy_start = Vec3::new(0.0, 0.0, 0.0);
        let enemy = app
            .world_mut()
            .spawn((
                Enemy,
                EntityState::Dead,
                Position(enemy_start),
                AggroRange(10.0),
                MovementStats { speed: 1.0 },
            ))
            .id();
        app.world_mut()
            .spawn((Player, Position(Vec3::new(0.5, 0.0, 0.0))))
            .id();

        app.update();

        let pos = app.world().entity(enemy).get::<Position>().unwrap().0;
        assert_eq!(pos, enemy_start, "dead enemy must not move");
    }

    #[test]
    fn living_enemies_still_chase() {
        let mut app = App::new();
        app.add_systems(Update, enemy_chase);

        let enemy_start = Vec3::new(0.0, 0.0, 0.0);
        let enemy = app
            .world_mut()
            .spawn((
                Enemy,
                EntityState::Idle,
                Position(enemy_start),
                AggroRange(10.0),
                MovementStats { speed: 0.5 },
            ))
            .id();
        app.world_mut()
            .spawn((Player, Position(Vec3::new(1.0, 0.0, 0.0))))
            .id();

        app.update();

        let pos = app.world().entity(enemy).get::<Position>().unwrap().0;
        assert!(
            pos.distance(enemy_start) > 0.0,
            "living enemy should have moved"
        );

        // Evita unused import warnings.
        let _ = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
    }
}
