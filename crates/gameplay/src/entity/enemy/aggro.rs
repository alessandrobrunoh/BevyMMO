//! Shared aggro helpers: leash, acquire origin, and threat-policy target pick.

use glam::Vec3;

use crate::EntityId;

/// Where the acquire circle is centered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AggroOrigin {
    /// The mob's current body. Trash that should pull as it patrols.
    #[default]
    Body,
    /// The spawn point. Stationary guards that ignore players who walk past
    /// a kited body outside the camp circle.
    Spawn,
}

/// How a mob notices a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AcquirePolicy {
    /// Living players inside the aggro circle become candidates.
    #[default]
    Proximity,
    /// Never proximity-pull. Combat starts only from a sticky/table target
    /// (first attacker, or accrued threat).
    Passive,
}

/// Who to fight among acquire candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThreatPolicy {
    /// Closest living candidate to the mob's body.
    #[default]
    Nearest,
    /// Keep `current` while they are still a candidate; otherwise nearest.
    Sticky,
    /// Highest threat amount; nearest if nobody has any yet.
    Table,
}

/// Authored aggro numbers plus the three policies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AggroProfile {
    pub acquire: AcquirePolicy,
    pub origin: AggroOrigin,
    pub threat: ThreatPolicy,
    pub aggro: f32,
    pub leash_aggro: f32,
}

/// One living player the selector can choose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreatCandidate {
    pub entity: EntityId,
    pub distance: f32,
}

/// Horizontal distance, matching the AI's xz aggro queries (height ignored).
pub fn horizontal_distance(a: Vec3, b: Vec3) -> f32 {
    Vec3::new(a.x - b.x, 0.0, a.z - b.z).length()
}

/// True when the mob has been dragged farther from spawn than `leash_aggro`.
///
/// `leash_aggro <= 0` means never leash (the kind has no camp to return to).
pub fn is_leashed(spawn: Vec3, position: Vec3, leash_aggro: f32) -> bool {
    leash_aggro > 0.0 && horizontal_distance(spawn, position) > leash_aggro
}

/// World position the acquire circle is drawn around.
pub fn acquire_center(origin: AggroOrigin, body: Vec3, spawn: Vec3) -> Vec3 {
    match origin {
        AggroOrigin::Body => body,
        AggroOrigin::Spawn => spawn,
    }
}

/// True when `player` is inside the horizontal circle of `aggro` around `center`.
pub fn in_acquire_radius(center: Vec3, player: Vec3, aggro: f32) -> bool {
    aggro > 0.0 && horizontal_distance(center, player) <= aggro
}

/// Whether this policy scans for players in the acquire circle.
pub fn acquires_by_proximity(policy: AcquirePolicy) -> bool {
    matches!(policy, AcquirePolicy::Proximity)
}

/// Pick a combat target from `candidates`.
///
/// - [`ThreatPolicy::Nearest`]: smallest `distance`, earlier candidate on ties.
/// - [`ThreatPolicy::Sticky`]: `current` if still in the set, else nearest.
/// - [`ThreatPolicy::Table`]: highest `threat_of`; nearest if every amount is 0.
pub fn select_target(
    policy: ThreatPolicy,
    candidates: &[ThreatCandidate],
    current: Option<EntityId>,
    threat_of: impl Fn(EntityId) -> f32,
) -> Option<EntityId> {
    if candidates.is_empty() {
        return None;
    }
    match policy {
        ThreatPolicy::Nearest => nearest_candidate(candidates),
        ThreatPolicy::Sticky => {
            if let Some(current) = current {
                if candidates.iter().any(|c| c.entity == current) {
                    return Some(current);
                }
            }
            nearest_candidate(candidates)
        }
        ThreatPolicy::Table => {
            highest_threat(candidates, threat_of).or_else(|| nearest_candidate(candidates))
        }
    }
}

fn nearest_candidate(candidates: &[ThreatCandidate]) -> Option<EntityId> {
    let mut best: Option<&ThreatCandidate> = None;
    for candidate in candidates {
        match best {
            Some(current) if candidate.distance >= current.distance => {}
            _ => best = Some(candidate),
        }
    }
    best.map(|c| c.entity)
}

fn highest_threat(
    candidates: &[ThreatCandidate],
    threat_of: impl Fn(EntityId) -> f32,
) -> Option<EntityId> {
    let mut best: Option<(EntityId, f32)> = None;
    for candidate in candidates {
        let amount = threat_of(candidate.entity);
        if amount <= 0.0 {
            continue;
        }
        match best {
            Some((_, best_amount)) if amount <= best_amount => {}
            _ => best = Some((candidate.entity, amount)),
        }
    }
    best.map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_the_leash_is_not_leashed() {
        let spawn = Vec3::ZERO;
        let position = Vec3::new(19.0, 4.0, 0.0);
        assert!(!is_leashed(spawn, position, 20.0));
    }

    #[test]
    fn past_the_leash_drops_combat() {
        let spawn = Vec3::ZERO;
        let position = Vec3::new(21.0, 0.0, 0.0);
        assert!(is_leashed(spawn, position, 20.0));
    }

    #[test]
    fn height_does_not_count_toward_leash() {
        let spawn = Vec3::ZERO;
        let position = Vec3::new(0.0, 50.0, 0.0);
        assert!(!is_leashed(spawn, position, 20.0));
    }

    #[test]
    fn non_positive_leash_never_triggers() {
        assert!(!is_leashed(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 0.0));
        assert!(!is_leashed(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), -1.0));
    }

    #[test]
    fn body_acquire_follows_a_dragged_mob_spawn_does_not() {
        let spawn = Vec3::ZERO;
        let body = Vec3::new(30.0, 0.0, 0.0);
        let player = Vec3::new(31.0, 0.0, 0.0);
        let aggro = 8.0;
        assert!(in_acquire_radius(
            acquire_center(AggroOrigin::Body, body, spawn),
            player,
            aggro,
        ));
        assert!(!in_acquire_radius(
            acquire_center(AggroOrigin::Spawn, body, spawn),
            player,
            aggro,
        ));
    }

    #[test]
    fn passive_never_proximity_pulls() {
        assert!(acquires_by_proximity(AcquirePolicy::Proximity));
        assert!(!acquires_by_proximity(AcquirePolicy::Passive));
    }

    fn candidate(id: u64, distance: f32) -> ThreatCandidate {
        ThreatCandidate {
            entity: EntityId::new(id),
            distance,
        }
    }

    #[test]
    fn nearest_picks_the_closest_candidate() {
        let candidates = [candidate(1, 8.0), candidate(2, 3.0), candidate(3, 5.0)];
        let picked = select_target(ThreatPolicy::Nearest, &candidates, None, |_| 0.0);
        assert_eq!(picked, Some(EntityId::new(2)));
    }

    #[test]
    fn sticky_keeps_current_even_when_not_nearest() {
        let candidates = [candidate(1, 2.0), candidate(2, 8.0)];
        let picked = select_target(
            ThreatPolicy::Sticky,
            &candidates,
            Some(EntityId::new(2)),
            |_| 0.0,
        );
        assert_eq!(picked, Some(EntityId::new(2)));
    }

    #[test]
    fn sticky_falls_back_to_nearest_when_current_is_gone() {
        let candidates = [candidate(1, 2.0), candidate(3, 8.0)];
        let picked = select_target(
            ThreatPolicy::Sticky,
            &candidates,
            Some(EntityId::new(2)),
            |_| 0.0,
        );
        assert_eq!(picked, Some(EntityId::new(1)));
    }

    #[test]
    fn table_picks_highest_threat_not_nearest() {
        let candidates = [candidate(1, 1.0), candidate(2, 9.0)];
        let picked = select_target(ThreatPolicy::Table, &candidates, None, |id| {
            if id.get() == 2 {
                50.0
            } else {
                10.0
            }
        });
        assert_eq!(picked, Some(EntityId::new(2)));
    }

    #[test]
    fn table_falls_back_to_nearest_when_nobody_has_threat() {
        let candidates = [candidate(1, 8.0), candidate(2, 3.0)];
        let picked = select_target(ThreatPolicy::Table, &candidates, None, |_| 0.0);
        assert_eq!(picked, Some(EntityId::new(2)));
    }

    #[test]
    fn empty_candidates_yield_no_target() {
        assert_eq!(
            select_target(ThreatPolicy::Nearest, &[], None, |_| 0.0),
            None
        );
        assert_eq!(
            select_target(ThreatPolicy::Sticky, &[], Some(EntityId::new(1)), |_| 0.0),
            None
        );
        assert_eq!(
            select_target(ThreatPolicy::Table, &[], None, |_| 99.0),
            None
        );
    }
}
