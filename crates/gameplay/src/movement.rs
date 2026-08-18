//! Point-to-point movement, shared by both sides of the wire.
//!
//! The server advances characters by calling [`step_towards`] on its tick; the
//! client calls the *same function* between server updates to predict where its
//! own character is going. That sharing is the point: with lightyear gone the
//! client no longer gets prediction for free, and two hand-written
//! implementations of "walk towards a point" would disagree in exactly the way
//! that makes a character rubber-band.
//!
//! Flat stepping remains useful for client reconciliation. Terrain stepping is
//! also defined here, but callers supply the world query and collision grid so
//! this crate remains independent of Bevy, filesystems, and storage.

use glam::Vec3;

/// Distance below which a character counts as having arrived.
///
/// Matches the threshold the Bevy server used, so the two agree on when
/// movement stops rather than leaving a character twitching on the spot.
pub const ARRIVAL_EPSILON: f32 = 0.001;

/// Outcome of advancing a character for one time step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    /// Still en route; the character is at this position.
    Moving(Vec3),
    /// Reached the target this step, and should stop.
    Arrived(Vec3),
}

/// Advances `position` towards `target` for `dt` seconds.
///
/// `speed` is in units per **second**. Note that the Bevy server stored speed as
/// units per *tick* at a fixed 60 Hz (`effective_speed.min(distance)`), which
/// only worked because the tick rate never varied. SpacetimeDB's scheduler does
/// not guarantee a fixed cadence — the interval is measured from the end of the
/// previous run, so a nominal 50 ms tick was measured at ~56 ms — hence the
/// explicit `dt` here. Converting an old value: `per_second = per_tick * 60.0`.
pub fn step_towards(position: Vec3, target: Vec3, speed: f32, dt: f32) -> Step {
    let offset = target - position;
    let distance = offset.length();

    if distance <= ARRIVAL_EPSILON {
        return Step::Arrived(target);
    }

    let travel = speed * dt;
    if travel >= distance {
        return Step::Arrived(target);
    }

    Step::Moving(position + offset / distance * travel)
}

/// Why a `move_to` request should be accepted or refused.
///
/// Charge and CastTime freeze the character. Channeling still accepts a
/// destination so movement can cancel an InterruptOnMove channel — the
/// tick, not the reducer, ends that cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementLock {
    None,
    CastTime,
    Charge,
    Channel,
}

/// Whether the player may issue a new destination.
///
/// `cc_blocks` covers Stun/Root. Channel is allowed so a held right-click
/// can interrupt; Charge is not, or the starter staff cancels itself.
pub fn movement_intent_allowed(lock: MovementLock, cc_blocks: bool) -> bool {
    if cc_blocks {
        return false;
    }
    match lock {
        MovementLock::None | MovementLock::Channel => true,
        MovementLock::CastTime | MovementLock::Charge => false,
    }
}

/// Horizontal facing implied by moving from `position` to `target`.
///
/// Returns `None` when the two are vertically aligned, in which case the caller
/// should keep the previous facing rather than snapping to an arbitrary one.
pub fn look_direction(position: Vec3, target: Vec3) -> Option<Vec3> {
    let flat = Vec3::new(target.x - position.x, 0.0, target.z - position.z);
    if flat.length() <= ARRIVAL_EPSILON {
        return None;
    }
    Some(flat.normalize_or_zero())
}

/// Outcome of a terrain-aware movement step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerrainStep {
    /// Entity reached its target; carry the resolved on-ground position.
    Arrived(Vec3),
    /// Entity moved one step toward the target.
    Moved(Vec3),
    /// Step was rejected by terrain or a blocker.
    Blocked,
    /// Target is not on a walkable surface.
    NoSurface,
}

/// Permissive vertical budget for spawn, persisted-position, and teleport recovery.
pub const SNAP_STEP_BUDGET: f32 = 5.0;

/// Default collision radius used while probing static world blockers.
const STEP_COLLISION_RADIUS: f32 = 0.45;

/// Snaps an entity onto the highest reachable ground surface at its X/Z point.
///
/// When the entity is stranded below terrain, this deliberately falls back to
/// the highest surface so spawn and persisted-position recovery cannot leave it
/// permanently unable to move.
pub fn snap_to_ground(
    position: &mut Vec3,
    surface_query: &crate::world::SurfaceQuery,
    max_step_height: f32,
) -> bool {
    let contact = surface_query
        .ground_at_reachable(position.x, position.z, position.y, max_step_height)
        .or_else(|| surface_query.ground_at(position.x, position.z));
    let Some(contact) = contact else {
        return false;
    };

    if (contact.height - position.y).abs() <= ARRIVAL_EPSILON {
        return false;
    }
    position.y = contact.height;
    true
}

/// Advances an entity toward an X/Z target across walkable terrain.
///
/// `max_travel` is the horizontal distance available for this simulation step;
/// callers with per-second speed should pass `speed * dt`. Height is always
/// resolved from the authoritative surface query, never from the target.
pub fn step_on_terrain(
    current: Vec3,
    target_x: f32,
    target_z: f32,
    max_travel: f32,
    surface_query: &crate::world::SurfaceQuery,
    collision_grid: &crate::world::CollisionGrid,
    max_step_height: f32,
) -> TerrainStep {
    let target_contact = match surface_query.ground_at(target_x, target_z) {
        Some(contact) => contact,
        None => return TerrainStep::NoSurface,
    };

    let dx = target_x - current.x;
    let dz = target_z - current.z;
    let horizontal_distance = (dx * dx + dz * dz).sqrt();
    if horizontal_distance <= ARRIVAL_EPSILON {
        return TerrainStep::Arrived(Vec3::new(target_x, target_contact.height, target_z));
    }

    let travel = max_travel.max(0.0).min(horizontal_distance);
    if travel <= 0.0 {
        return TerrainStep::Blocked;
    }
    let nx = dx / horizontal_distance;
    let nz = dz / horizontal_distance;

    // Recover from a stale or externally authored position that is already
    // inside a blocker. This keeps the character visibly outside the wall
    // instead of merely preventing further movement while embedded.
    if collision_grid.is_blocked([current.x, current.y, current.z], STEP_COLLISION_RADIUS) {
        if let Some(position) = recover_from_blocker(
            current,
            nx,
            nz,
            surface_query,
            collision_grid,
            max_step_height,
        ) {
            return TerrainStep::Moved(position);
        }
        return TerrainStep::Blocked;
    }

    if let Some(position) = try_terrain_step(
        current,
        nx,
        nz,
        travel,
        surface_query,
        collision_grid,
        max_step_height,
    ) {
        return TerrainStep::Moved(position);
    }

    let (first, second) = if nx.abs() >= nz.abs() {
        ((nx, 0.0), (0.0, nz))
    } else {
        ((0.0, nz), (nx, 0.0))
    };
    for (step_x, step_z) in [first, second] {
        if let Some(position) = try_terrain_step(
            current,
            step_x,
            step_z,
            travel,
            surface_query,
            collision_grid,
            max_step_height,
        ) {
            return TerrainStep::Moved(position);
        }
    }

    TerrainStep::Blocked
}

fn recover_from_blocker(
    current: Vec3,
    direction_x: f32,
    direction_z: f32,
    surface_query: &crate::world::SurfaceQuery,
    collision_grid: &crate::world::CollisionGrid,
    max_step_height: f32,
) -> Option<Vec3> {
    let length = (direction_x * direction_x + direction_z * direction_z).sqrt();
    if length <= ARRIVAL_EPSILON {
        return None;
    }

    // Walk opposite the requested direction in small increments until the
    // circle footprint is clear. The bound is deliberately short: it repairs
    // penetration caused by a stale position without teleporting a player
    // across a room.
    for distance in (1..=16).map(|step| step as f32 * 0.1) {
        let x = current.x - direction_x / length * distance;
        let z = current.z - direction_z / length * distance;
        let contact = surface_query.ground_at_reachable(x, z, current.y, max_step_height)?;
        let candidate = Vec3::new(x, contact.height, z);
        if !collision_grid.is_blocked([candidate.x, candidate.y, candidate.z], STEP_COLLISION_RADIUS) {
            return Some(candidate);
        }
    }
    None
}

fn try_terrain_step(
    current: Vec3,
    direction_x: f32,
    direction_z: f32,
    travel: f32,
    surface_query: &crate::world::SurfaceQuery,
    collision_grid: &crate::world::CollisionGrid,
    max_step_height: f32,
) -> Option<Vec3> {
    let length = (direction_x * direction_x + direction_z * direction_z).sqrt();
    if length <= ARRIVAL_EPSILON {
        return None;
    }

    let next_x = current.x + direction_x / length * travel;
    let next_z = current.z + direction_z / length * travel;
    let contact = surface_query.ground_at_reachable(next_x, next_z, current.y, max_step_height)?;
    let candidate = Vec3::new(next_x, contact.height, next_z);

    (!collision_grid.is_blocked(
        [candidate.x, candidate.y, candidate.z],
        STEP_COLLISION_RADIUS,
    ))
    .then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{
        CollisionGrid, MapBounds, MapManifest, SurfaceBounds, SurfaceKind, SurfaceQuery,
        WalkableSurface, WorldMetrics,
    };

    fn flat_world(height: f32) -> (SurfaceQuery, CollisionGrid) {
        let manifest = MapManifest {
            version: 2,
            map_id: "movement_test".to_string(),
            display_name: "Movement Test".to_string(),
            bounds: MapBounds {
                min_x: -10.0,
                max_x: 10.0,
                min_z: -10.0,
                max_z: 10.0,
            },
            terrain: Default::default(),
            props: vec![],
            world_metrics: Some(WorldMetrics::default()),
            surfaces: vec![WalkableSurface {
                id: "ground".to_string(),
                kind: SurfaceKind::Flat,
                object: None,
                bounds: Some(SurfaceBounds {
                    min_x: -10.0,
                    max_x: 10.0,
                    min_z: -10.0,
                    max_z: 10.0,
                }),
                height: Some(height),
                min_height: None,
                max_height: None,
                grid_size: None,
                size: None,
                purpose: None,
                heightfield: None,
                walkable_mesh: None,
                layer: None,
                max_slope_deg: None,
            }],
            traversals: vec![],
            blockers: vec![],
            test_route: vec![],
            test_checklist: vec![],
            mountain_switchback_test: None,
            distant_plateau_test: None,
        };
        (
            SurfaceQuery::from_manifest(&manifest),
            CollisionGrid::build(&manifest),
        )
    }

    #[test]
    fn moves_along_the_line_to_the_target() {
        let step = step_towards(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), 2.0, 0.5);
        assert_eq!(step, Step::Moving(Vec3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn never_overshoots_the_target() {
        // One second at 100 u/s covers far more than the 3 units available.
        let step = step_towards(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0), 100.0, 1.0);
        assert_eq!(step, Step::Arrived(Vec3::new(3.0, 0.0, 0.0)));
    }

    #[test]
    fn arrives_when_already_on_target() {
        let target = Vec3::new(4.0, 1.0, -2.0);
        assert_eq!(
            step_towards(target, target, 5.0, 0.05),
            Step::Arrived(target)
        );
    }

    #[test]
    fn a_longer_step_covers_proportionally_more_ground() {
        // The property that matters for prediction: splitting a step in two
        // must land in the same place as taking it whole, so a client ticking
        // at frame rate agrees with a server ticking at 20 Hz.
        let target = Vec3::new(10.0, 0.0, 0.0);
        let Step::Moving(once) = step_towards(Vec3::ZERO, target, 2.0, 1.0) else {
            panic!("expected to still be moving");
        };
        let Step::Moving(half) = step_towards(Vec3::ZERO, target, 2.0, 0.5) else {
            panic!("expected to still be moving");
        };
        let Step::Moving(twice) = step_towards(half, target, 2.0, 0.5) else {
            panic!("expected to still be moving");
        };
        assert!((once - twice).length() < 1e-5, "{once} vs {twice}");
    }

    #[test]
    fn look_direction_ignores_height() {
        let dir = look_direction(Vec3::ZERO, Vec3::new(0.0, 99.0, 5.0)).expect("has a facing");
        assert_eq!(dir, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn look_direction_is_none_when_only_height_differs() {
        assert_eq!(look_direction(Vec3::ZERO, Vec3::new(0.0, 5.0, 0.0)), None);
    }

    #[test]
    fn terrain_step_snaps_then_follows_authoritative_ground_height() {
        let (surfaces, collision) = flat_world(2.0);
        let mut current = Vec3::new(0.0, 0.0, 0.0);
        assert!(snap_to_ground(&mut current, &surfaces, 0.45));

        assert_eq!(
            step_on_terrain(current, 2.0, 0.0, 0.5, &surfaces, &collision, 0.45),
            TerrainStep::Moved(Vec3::new(0.5, 2.0, 0.0))
        );
    }

    #[test]
    fn terrain_step_rejects_targets_outside_walkable_surfaces() {
        let (surfaces, collision) = flat_world(0.0);
        assert_eq!(
            step_on_terrain(Vec3::ZERO, 20.0, 0.0, 1.0, &surfaces, &collision, 0.45),
            TerrainStep::NoSurface
        );
    }

    #[test]
    fn charge_and_cast_time_block_movement_intent() {
        assert!(!movement_intent_allowed(MovementLock::Charge, false));
        assert!(!movement_intent_allowed(MovementLock::CastTime, false));
        assert!(movement_intent_allowed(MovementLock::None, false));
        assert!(movement_intent_allowed(MovementLock::Channel, false));
    }

    #[test]
    fn stun_blocks_even_when_not_casting() {
        assert!(!movement_intent_allowed(MovementLock::None, true));
        assert!(!movement_intent_allowed(MovementLock::Channel, true));
    }

    #[test]
    fn charge_is_not_treated_as_channel() {
        assert_ne!(
            movement_intent_allowed(MovementLock::Charge, false),
            movement_intent_allowed(MovementLock::Channel, false)
        );
    }
}
