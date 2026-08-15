//! Point-to-point movement, shared by both sides of the wire.
//!
//! The server advances characters by calling [`step_towards`] on its tick; the
//! client calls the *same function* between server updates to predict where its
//! own character is going. That sharing is the point: with lightyear gone the
//! client no longer gets prediction for free, and two hand-written
//! implementations of "walk towards a point" would disagree in exactly the way
//! that makes a character rubber-band.
//!
//! Terrain following and collision are deliberately absent — they belong with
//! the world data, which only the server holds. This is the flat-ground case
//! that `bevymmo_server::player_movement` falls back to when no surface is
//! loaded.

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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(step_towards(target, target, 5.0, 0.05), Step::Arrived(target));
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
}
