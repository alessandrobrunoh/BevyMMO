//! Shared aggro helpers. Slice 1 is leash-from-spawn; acquire/threat policies
//! land in a later slice.

use glam::Vec3;

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
}
