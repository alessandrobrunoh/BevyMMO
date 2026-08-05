//! Definizione di spawn dell'Enemy.
//!
//! Usando `spawn_entity::<Enemy>()` l'enemy viene creato con `GameEntity`,
//! `Health`, `Position`, `EntityColor`, il bundle qui sotto e
//! `Replicate::to_clients(NetworkTarget::All)` in automatico, quindi è
//! subito sincronizzato sul network.

use bevy::prelude::*;

use super::components::{AggroRange, Enemy, EnemyAttack};
use crate::plugins::entity::components::{Health, Stats};
use crate::plugins::entity::definition::EntityDefinition;

impl EntityDefinition for Enemy {
    fn name() -> &'static str {
        "Enemy"
    }

    fn bundle() -> impl Bundle {
        (Enemy, AggroRange::default(), EnemyAttack::default())
    }

    fn initial_position() -> Vec3 {
        Vec3::new(5.0, 0.0, 5.0)
    }

    fn initial_color() -> Color {
        Color::srgb(0.8, 0.2, 0.2)
    }

    fn health() -> Health {
        Health::new(50.0)
    }

    fn stats() -> Stats {
        Stats::with_combat_values(0.08, 18.0, 50.0, 40.0, 2.0, 10.0)
    }
}
