//! Contratto di definizione di un'entità di gioco.
//!
//! `EntityDefinition` NON è un trait OOP "è-un": è un contratto di *dato*
//! che ogni entità concreta implementa per centralizzare lo spawn e la
//! configurazione di rete. In Bevy è idiomatico avere componenti marker
//! + bundle di dati, non polimorfismo runtime. Questo trait dichiara
//! "come si costruisce" un'entità e quali componenti di rete vuole.

use bevy::prelude::*;
use lightyear::prelude::NetworkTarget;

use crate::plugins::entity::components::EntityKind;
use crate::stats::components::StatsBundleData;

/// Ogni entità di gioco implementa questo trait. L'helper `spawn_entity::<T>()`
/// lo usa per costruire l'entità in modo uniforme, applicando automaticamente
/// `GameEntity`, i componenti statistici (`MovementStats`, `CombatStats`, `VitalStats`),
/// `Position`, `EntityColor` e la replicazione lightyear. Così ogni nuova entità
/// è automaticamente sincronizzata sul network senza configurazione manuale.
pub trait EntityDefinition: Component {
    /// Nome leggibile (logging, debug).
    fn name() -> &'static str;

    /// Bundle di componenti identità/dati specifiche (solo marker + componenti
    /// proprie di questa entità, NON `Position`/`EntityColor`/statistiche che sono
    /// gestite dal sistema di spawn centrale).
    fn bundle() -> impl Bundle;

    /// Posizione iniziale. Default `Vec3::ZERO`.
    fn initial_position() -> Vec3 {
        Vec3::ZERO
    }

    /// Colore iniziale. Default grigio neutro.
    fn initial_color() -> Color {
        Color::srgb(0.5, 0.5, 0.5)
    }

    /// Tipo di entità per targeting/UI. Default `Neutral`.
    fn entity_kind() -> EntityKind {
        EntityKind::Neutral
    }

    /// Statistiche iniziali di movimento, combattimento e vitali.
    fn stats() -> StatsBundleData {
        StatsBundleData {
            movement: crate::stats::components::MovementStats { speed: 0.15 },
            combat: crate::stats::components::CombatStats {
                attack_power: 10.0,
                armor: 0.0,
            },
            vital: crate::stats::components::VitalStats {
                current_health: 100.0,
                max_health: 100.0,
                max_mana: 100.0,
                mana_regeneration: 5.0,
            },
        }
    }

    /// Target di replicazione lightyear di default. Override solo se serve
    /// un target diverso (es. `NetworkTarget::AllExceptSingle(peer)`).
    fn replication_target() -> NetworkTarget {
        NetworkTarget::All
    }
}
