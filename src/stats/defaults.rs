//! Profili statistici di default per Player ed Enemy.
//!
//! Centralizzare i default qui mantiene coerenza tra spawn, persistence
//! (backfill) e test. I valori ricalcolano quelli attualmente hard-coded
//! nelle rispettive `impl EntityDefinition`.

use crate::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};

/// Profilo statistico di default del Player.
pub fn player_defaults() -> StatsBundleData {
    StatsBundleData {
        movement: MovementStats { speed: 0.15 },
        combat: CombatStats {
            attack_power: 10.0,
            armor: 25.0,
        },
        vital: VitalStats {
            current_health: 100.0,
            max_health: 100.0,
            max_mana: 100.0,
            mana_regeneration: 5.0,
        },
    }
}

/// Profilo statistico di default dell'Enemy.
pub fn enemy_defaults() -> StatsBundleData {
    StatsBundleData {
        movement: MovementStats { speed: 0.08 },
        combat: CombatStats {
            attack_power: 20.0,
            armor: 10.0,
        },
        vital: VitalStats {
            current_health: 50.0,
            max_health: 50.0,
            max_mana: 40.0,
            mana_regeneration: 2.0,
        },
    }
}

/// Profilo statistico di default del Dummy.
///
/// Il Dummy è un bersaglio statico con HP enormi, usato per testare
/// sistema danni, UI targeting e spell. Non si muove e non ha stats offensive.
pub fn dummy_defaults() -> StatsBundleData {
    StatsBundleData {
        movement: MovementStats { speed: 0.0 },
        combat: CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        },
        vital: VitalStats {
            current_health: 1_000_000_000.0,
            max_health: 1_000_000_000.0,
            max_mana: 0.0,
            mana_regeneration: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_defaults_start_at_full_health() {
        let stats = player_defaults();
        assert_eq!(stats.vital.current_health, stats.vital.max_health);
    }

    #[test]
    fn enemy_defaults_start_at_full_health() {
        let stats = enemy_defaults();
        assert_eq!(stats.vital.current_health, stats.vital.max_health);
    }

    #[test]
    fn dummy_defaults_start_at_full_health() {
        let stats = dummy_defaults();
        assert_eq!(stats.vital.current_health, stats.vital.max_health);
    }

    #[test]
    fn dummy_defaults_have_zero_speed() {
        let stats = dummy_defaults();
        assert_eq!(stats.movement.speed, 0.0);
    }

    #[test]
    fn dummy_defaults_have_huge_hp() {
        let stats = dummy_defaults();
        assert_eq!(stats.vital.max_health, 1_000_000_000.0);
    }
}
