//! Componenti condivise da tutte le entità di gioco.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Marker per qualsiasi entità di gioco (Player, Enemy, NPC, ...).
///
/// Il nome evita ambiguità con `bevy::ecs::entity::Entity`, che identifica
/// un'istanza ECS, non una categoria di gameplay.
#[derive(Component, Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GameEntity;

/// Stato comportamentale condiviso e replicato di un'entità di gioco.
///
/// Le transizioni che cambiano il gameplay devono essere decise dal server.
/// `Dead` è terminale finché un sistema esplicito di respawn non assegna un
/// nuovo stato.
#[derive(
    Component, Debug, Default, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq,
)]
#[reflect(Component)]
pub enum EntityState {
    #[default]
    Idle,
    Moving,
    Dead,
}

impl EntityState {
    pub fn is_dead(self) -> bool {
        self == Self::Dead
    }
}

/// Salute condivisa. Sottratta da `damage` systems, ripristinata da `heal`.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }
}

/// Statistiche di combattimento e movimento condivise.
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct Stats {
    pub speed: f32,
    pub damage: f32,
    pub max_health: f32,
    pub max_mana: f32,
    pub mana_regeneration: f32,
    pub armor: f32,
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct PlayerName(pub String);

impl Default for Stats {
    fn default() -> Self {
        Self {
            speed: 0.15,
            damage: 10.0,
            max_health: 100.0,
            max_mana: 100.0,
            mana_regeneration: 5.0,
            armor: 0.0,
        }
    }
}

impl Stats {
    pub fn new(speed: f32, damage: f32) -> Self {
        Self {
            speed,
            damage,
            ..Self::default()
        }
    }

    pub fn with_combat_values(
        speed: f32,
        damage: f32,
        max_health: f32,
        max_mana: f32,
        mana_regeneration: f32,
        armor: f32,
    ) -> Self {
        Self {
            speed,
            damage,
            max_health,
            max_mana,
            mana_regeneration,
            armor,
        }
    }

    /// Returns the fraction of incoming damage prevented by this armor value.
    pub fn damage_reduction(&self) -> f32 {
        let armor = self.armor.max(0.0);
        (armor / (armor + 100.0)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Stats;

    #[test]
    fn armor_reduction_uses_the_expected_curve() {
        let stats = Stats::with_combat_values(0.15, 10.0, 100.0, 100.0, 5.0, 100.0);

        assert_eq!(stats.damage_reduction(), 0.5);
    }

    #[test]
    fn armor_reduction_clamps_negative_armor_and_never_exceeds_one() {
        let negative = Stats::with_combat_values(0.15, 10.0, 100.0, 100.0, 5.0, -50.0);
        let very_high = Stats::with_combat_values(0.15, 10.0, 100.0, 100.0, 5.0, 1.0e30);

        assert_eq!(negative.damage_reduction(), 0.0);
        assert_eq!(very_high.damage_reduction(), 1.0);
    }
}
