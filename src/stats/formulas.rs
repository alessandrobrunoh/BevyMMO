//! Formule di combattimento condivise.
//!
//! Funzioni pure, facili da testare, usate dai sistemi stats e dalle spell.

use crate::stats::components::CombatStats;

/// Danno effettivo dopo la riduzione da armatura del bersaglio.
///
/// `raw_damage * (1 - armor_damage_reduction)`, clamped a `>= 0`.
/// Danno negativo in input non cura mai il bersaglio.
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
