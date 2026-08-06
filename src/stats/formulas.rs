//! Shared combat formulas.
//!
//! Pure functions, easy to test, used by stats systems and spells.

use crate::stats::components::CombatStats;

/// Effective damage after target armor reduction.
///
/// `raw_damage * (1 - armor_damage_reduction)`, clamped to `>= 0`.
/// Negative damage input never heals the target.
pub fn damage_after_armor(raw_damage: f32, target_combat: &CombatStats) -> f32 {
    (raw_damage * (1.0 - target_combat.armor_damage_reduction())).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_respects_target_armor() {
        let target = CombatStats {
            attack_power: 0.0,
            armor: 100.0,
        };
        // 100 armor = 50% reduction
        assert_eq!(damage_after_armor(10.0, &target), 5.0);
    }

    #[test]
    fn damage_never_heals_or_goes_below_zero() {
        let target = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
        assert_eq!(damage_after_armor(-10.0, &target), 0.0);
    }

    #[test]
    fn damage_with_zero_armor_is_unchanged() {
        let target = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
        assert_eq!(damage_after_armor(25.0, &target), 25.0);
    }
}
