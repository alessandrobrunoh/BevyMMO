//! Threat amount from a hit. Shared by the module's `accrue_threat` so a
//! tank's `threat_generation` cannot drift from a second handwritten formula.

/// Threat granted for `effective` (post-armor) damage.
///
/// `threat_generation` is the source's combat stat; `1.0` is a normal hit.
/// Negative products are clamped to zero so a typo cannot heal threat.
pub fn threat_from_damage(effective: f32, threat_generation: f32) -> f32 {
    (effective * threat_generation).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_damage_scales_with_threat_generation() {
        let effective = 40.0;
        let normal = threat_from_damage(effective, 1.0);
        let tank = threat_from_damage(effective, 2.0);
        assert_eq!(normal, 40.0);
        assert_eq!(tank, 80.0);
        assert!(tank > normal);
    }

    #[test]
    fn zero_or_negative_inputs_grant_no_threat() {
        assert_eq!(threat_from_damage(0.0, 2.0), 0.0);
        assert_eq!(threat_from_damage(-10.0, 2.0), 0.0);
        assert_eq!(threat_from_damage(10.0, 0.0), 0.0);
        assert_eq!(threat_from_damage(10.0, -1.0), 0.0);
    }
}
