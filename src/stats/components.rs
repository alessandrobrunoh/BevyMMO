//! Componenti runtime delle statistiche di gioco.
//!
//! Le statistiche sono divise in tre componenti ECS separate per mantenere
//! le query granulari e ridurre l'accoppiamento:
//! - [`MovementStats`] — velocità e parametri di movimento
//! - [`CombatStats`] — potere d'attacco e armatura
//! - [`VitalStats`] — salute, mana e rigenerazione
//!
//! [`StatsBundleData`] è un aggregato DTO usato ai confini di spawn,
//! configurazione e persistenza; non sostituisce i componenti ECS a runtime.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Statistiche di movimento.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct MovementStats {
    pub speed: f32,
}

/// Statistiche di combattimento.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct CombatStats {
    pub attack_power: f32,
    pub armor: f32,
}

impl CombatStats {
    /// Frazione di danno incoming prevenuta dall'armatura.
    ///
    /// Formula: `armor / (armor + 100)`, clamped `[0, 1]`.
    /// Valori negativi di armor vengono trattati come 0.
    pub fn armor_damage_reduction(&self) -> f32 {
        let armor = self.armor.max(0.0);
        (armor / (armor + 100.0)).clamp(0.0, 1.0)
    }
}

/// Statistiche vitali: salute corrente/massima, mana e rigenerazione.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct VitalStats {
    pub current_health: f32,
    pub max_health: f32,
    pub max_mana: f32,
    pub mana_regeneration: f32,
}

impl VitalStats {
    /// True se la salute corrente è esaurita.
    pub fn is_dead(&self) -> bool {
        self.current_health <= 0.0
    }

    /// Rettifica `current_health` per non superare `max_health` e non scendere
    /// sotto zero. Utile dopo modifiche a `max_health` o caricamento da DB.
    pub fn clamp_health(&mut self) {
        self.current_health = self.current_health.clamp(0.0, self.max_health);
    }
}

/// Aggregato DTO di tutte le statistiche.
///
/// Usato per:
/// - default di entità (`EntityDefinition`, definizioni enemy/spell)
/// - serializzazione/persistenza
/// - spawn helper
///
/// A runtime, i valori vivono nei tre componenti ECS separati; usa
/// [`StatsBundleData::into_components`] per ottenere la tupla di componenti
/// da inserire in un'entità.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct StatsBundleData {
    pub movement: MovementStats,
    pub combat: CombatStats,
    pub vital: VitalStats,
}

impl StatsBundleData {
    /// Costruisce il bundle dai tre componenti runtime.
    pub fn from_components(
        movement: &MovementStats,
        combat: &CombatStats,
        vital: &VitalStats,
    ) -> Self {
        Self {
            movement: *movement,
            combat: *combat,
            vital: *vital,
        }
    }

    /// Decompone il DTO nella tupla di componenti ECS.
    pub fn into_components(self) -> (MovementStats, CombatStats, VitalStats) {
        (self.movement, self.combat, self.vital)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_reduction_uses_the_expected_curve() {
        let combat = CombatStats {
            attack_power: 10.0,
            armor: 100.0,
        };
        assert_eq!(combat.armor_damage_reduction(), 0.5);
    }

    #[test]
    fn armor_reduction_clamps_negative_and_high_values() {
        let negative = CombatStats {
            attack_power: 10.0,
            armor: -50.0,
        };
        let very_high = CombatStats {
            attack_power: 10.0,
            armor: 1.0e30,
        };
        assert_eq!(negative.armor_damage_reduction(), 0.0);
        assert_eq!(very_high.armor_damage_reduction(), 1.0);
    }

    #[test]
    fn vital_stats_clamp_health_respects_bounds() {
        let mut vital = VitalStats {
            current_health: 150.0,
            max_health: 100.0,
            max_mana: 50.0,
            mana_regeneration: 5.0,
        };
        vital.clamp_health();
        assert_eq!(vital.current_health, 100.0);

        vital.current_health = -10.0;
        vital.clamp_health();
        assert_eq!(vital.current_health, 0.0);
    }

    #[test]
    fn bundle_data_roundtrips_through_components() {
        let movement = MovementStats { speed: 0.15 };
        let combat = CombatStats {
            attack_power: 10.0,
            armor: 25.0,
        };
        let vital = VitalStats {
            current_health: 80.0,
            max_health: 100.0,
            max_mana: 50.0,
            mana_regeneration: 5.0,
        };

        let bundle = StatsBundleData::from_components(&movement, &combat, &vital);
        let (m, c, v) = bundle.into_components();
        assert_eq!(m, movement);
        assert_eq!(c, combat);
        assert_eq!(v, vital);
    }
}
