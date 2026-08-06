//! Pure target-selection helpers for the boss AI.
//!
//! These functions take the threat table and the list of living player
//! positions and resolve which entity/cluster an ability should hit. Keeping
//! them pure (no ECS mutation) makes them trivial to unit-test and lets the
//! rotation driver pick a strategy per ability without re-querying.

use bevy::math::Vec3;
use bevy::prelude::Entity;

use bevymmo_shared::entity::boss::components::ThreatTable;

/// A living player's id and position, the input shape for selection helpers.
pub struct PlayerRef {
    pub entity: Entity,
    pub position: Vec3,
}

/// Returns the highest-threat living player, or `None` if no living player has
/// accrued threat.
///
/// Ties are broken by threat insertion order (the iteration order of the
/// `HashMap`), which is deterministic within a run but not stable across
/// rebuilds; that's acceptable for AI flavour.
///
/// # Example
/// ```rust,ignore
/// if let Some(target) = highest_threat(&threat, &players) {
///     aim_breath_at(target);
/// }
/// ```
pub fn highest_threat<'a>(threat: &ThreatTable, players: &'a [PlayerRef]) -> Option<&'a PlayerRef> {
    players
        .iter()
        .filter_map(|player| {
            threat
                .entries
                .get(&player.entity)
                .map(|amount| (player, *amount))
        })
        .max_by(|left, right| {
            left.1
                .partial_cmp(&right.1)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
        .map(|(player, _)| player)
}

/// Returns the farthest living player from `origin`, or `None` if no players.
pub fn farthest_target<'a>(players: &'a [PlayerRef], origin: Vec3) -> Option<&'a PlayerRef> {
    players.iter().max_by(|left, right| {
        left.position
            .distance_squared(origin)
            .partial_cmp(&right.position.distance_squared(origin))
            .unwrap_or(core::cmp::Ordering::Equal)
    })
}

/// Returns the nearest living player to `origin`, or `None` if no players.
///
/// Used as the fallback target when no player has accrued threat yet, so the
/// boss starts fighting the moment a player enters the arena ring.
pub fn nearest_player<'a>(players: &'a [PlayerRef], origin: Vec3) -> Option<&'a PlayerRef> {
    players.iter().min_by(|left, right| {
        left.position
            .distance_squared(origin)
            .partial_cmp(&right.position.distance_squared(origin))
            .unwrap_or(core::cmp::Ordering::Equal)
    })
}

/// Resolves the boss's primary target: the highest-threat living player, or —
/// if no one has threat yet — the nearest living player to `origin`.
///
/// This guarantees the boss engages immediately on arena enter instead of
/// standing idle until a player lands the first hit.
pub fn main_target<'a>(
    threat: &ThreatTable,
    players: &'a [PlayerRef],
    origin: Vec3,
) -> Option<&'a PlayerRef> {
    highest_threat(threat, players).or_else(|| nearest_player(players, origin))
}

/// Returns the centroid of the `n` most clustered living players.
///
/// Strategy: try every combination of `n` players, pick the group whose
/// bounding-sphere radius (max pairwise distance / 2) is smallest, and return
/// that group. The caller uses the centroid to place AoE circles.
///
/// With the small `n` and modest player counts of an MMO encounter this O(C(p,
/// n)) brute force is cheap and deterministic.
///
/// Returns `None` if fewer than `n` players are alive.
pub fn densest_cluster(players: &[PlayerRef], n: usize) -> Option<Vec3> {
    if players.len() < n || n == 0 {
        return None;
    }
    let indices: Vec<usize> = (0..players.len()).collect();
    let mut best: Option<(f32, Vec3)> = None;

    for combo in combinations(&indices, n) {
        let spread = max_pairwise_distance(&combo, players);
        if spread.is_nan() {
            continue;
        }
        let centroid = combo.iter().map(|&i| players[i].position).sum::<Vec3>() / n as f32;
        match &best {
            None => best = Some((spread, centroid)),
            Some((best_spread, _)) if spread < *best_spread => best = Some((spread, centroid)),
            _ => {}
        }
    }
    best.map(|(_, centroid)| centroid)
}

/// Heap's algorithm producing all length-`k` index combinations.
fn combinations(indices: &[usize], k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut current: Vec<usize> = Vec::with_capacity(k);
    combine_recursive(indices, 0, k, &mut current, &mut out);
    out
}

fn combine_recursive(
    indices: &[usize],
    start: usize,
    k: usize,
    current: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if current.len() == k {
        out.push(current.clone());
        return;
    }
    for i in start..indices.len() {
        current.push(indices[i]);
        combine_recursive(indices, i + 1, k, current, out);
        current.pop();
    }
}

fn max_pairwise_distance(combo: &[usize], players: &[PlayerRef]) -> f32 {
    let mut max_sq = 0.0_f32;
    for a in 0..combo.len() {
        for b in (a + 1)..combo.len() {
            let d = players[combo[a]]
                .position
                .distance_squared(players[combo[b]].position);
            if d > max_sq {
                max_sq = d;
            }
        }
    }
    max_sq.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::World;
    use std::collections::HashMap;

    fn player(entity: Entity, x: f32, z: f32) -> PlayerRef {
        PlayerRef {
            entity,
            position: Vec3::new(x, 0.0, z),
        }
    }

    /// Reserves `count` fresh entity ids from a throwaway world so the tests
    /// don't depend on `Entity::from_raw` (Bevy 0.19 doesn't expose it).
    fn reserve_entities(count: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..count).map(|_| world.spawn_empty().id()).collect()
    }

    fn threat_with(entries: &[(Entity, f32)]) -> ThreatTable {
        let mut map = HashMap::new();
        for (entity, amount) in entries {
            map.insert(*entity, *amount);
        }
        ThreatTable { entries: map }
    }

    #[test]
    fn highest_threat_picks_the_max_threat_among_living_players() {
        let ids = reserve_entities(3);
        let threat = threat_with(&[(ids[0], 10.0), (ids[1], 50.0), (ids[2], 20.0)]);
        let players = vec![
            player(ids[0], 0.0, 0.0),
            player(ids[1], 1.0, 0.0),
            player(ids[2], 2.0, 0.0),
        ];

        let target = highest_threat(&threat, &players).unwrap();
        assert_eq!(target.entity, ids[1]);
    }

    #[test]
    fn highest_threat_ignores_players_with_no_threat() {
        let ids = reserve_entities(2);
        let threat = threat_with(&[(ids[0], 10.0)]);
        let players = vec![player(ids[0], 0.0, 0.0), player(ids[1], 1.0, 0.0)];

        let target = highest_threat(&threat, &players).unwrap();
        assert_eq!(target.entity, ids[0]);
    }

    #[test]
    fn highest_threat_returns_none_when_no_living_player_has_threat() {
        let ids = reserve_entities(2);
        let threat = threat_with(&[(ids[0], 10.0)]);
        let players = vec![player(ids[1], 0.0, 0.0)];

        assert!(highest_threat(&threat, &players).is_none());
    }

    #[test]
    fn farthest_target_returns_none_for_empty_input() {
        assert!(farthest_target(&[], Vec3::ZERO).is_none());
    }

    #[test]
    fn farthest_target_picks_the_most_distant_player() {
        let ids = reserve_entities(3);
        let players = vec![
            player(ids[0], 0.0, 0.0),
            player(ids[1], 5.0, 0.0),
            player(ids[2], 2.0, 0.0),
        ];
        let target = farthest_target(&players, Vec3::ZERO).unwrap();
        assert_eq!(target.entity, ids[1]);
    }

    #[test]
    fn densest_cluster_returns_none_when_too_few_players() {
        let ids = reserve_entities(1);
        let players = vec![player(ids[0], 0.0, 0.0)];
        assert!(densest_cluster(&players, 2).is_none());
    }

    #[test]
    fn densest_cluster_picks_the_tightest_pair() {
        // Two clusters: a tight pair at (0,0)/(0.5,0) and a loose point at (10,0).
        // The tightest pair must win and its centroid land at (0.25, 0, 0).
        let ids = reserve_entities(3);
        let players = vec![
            player(ids[0], 0.0, 0.0),
            player(ids[1], 0.5, 0.0),
            player(ids[2], 10.0, 0.0),
        ];
        let centroid = densest_cluster(&players, 2).unwrap();
        assert!((centroid - Vec3::new(0.25, 0.0, 0.0)).length() < 1e-5);
    }
}
