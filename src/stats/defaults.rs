//! Default statistical profiles for Player and Enemy.
//!
//! Centralizing defaults here maintains consistency across spawn, persistence
//! (backfill), and testing. The values mirror those currently defined
//! in respective `impl EntityDefinition`.

use crate::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};

/// Default statistical profile for Player.
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

/// Default statistical profile for Enemy.
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

/// Default statistical profile for the dragon boss.
///
/// The boss has slow server-authoritative chase movement, heavy HP for a
/// multi-phase encounter, and solid armor.
pub fn boss_defaults() -> StatsBundleData {
    StatsBundleData {
        movement: MovementStats { speed: 0.05 },
        combat: CombatStats {
            attack_power: 28.0,
            armor: 30.0,
        },
        vital: VitalStats {
            current_health: 6000.0,
            max_health: 6000.0,
            max_mana: 0.0,
            mana_regeneration: 0.0,
        },
    }
}

/// Default statistical profile for Dummy.
///
/// The Dummy is a static target with huge HP, used for testing
/// damage systems, UI targeting, and spells. It does not move and has no offensive stats.
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
    fn boss_defaults_start_at_full_health() {
        let stats = boss_defaults();
        assert_eq!(stats.vital.current_health, stats.vital.max_health);
    }

    #[test]
    fn boss_defaults_have_high_hp_pool() {
        let stats = boss_defaults();
        assert_eq!(stats.vital.max_health, 6000.0);
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
