//! Contratto di definizione di un'entità di gioco.
//!
//! `EntityDefinition` NON è un trait OOP "è-un": è un contratto di *dato*
//! che ogni entità concreta implementa per centralizzare lo spawn e la
//! configurazione di rete. In Bevy è idiomatico avere componenti marker
//! + bundle di dati, non polimorfismo runtime. Questo trait dichiara
//! "come si costruisce" un'entità e quali componenti di rete vuole.

use bevy::prelude::*;
use lightyear::prelude::NetworkTarget;

use super::components::{Health, Stats};

/// Ogni entità di gioco implementa questo trait. L'helper `spawn_entity::<T>()`
/// lo usa per costruire l'entità in modo uniforme, applicando automaticamente
/// `GameEntity`, `Health`, `Position`, `EntityColor` e la replicazione
/// lightyear. Così ogni nuova entità è automaticamente sincronizzata sul
/// network senza configurazione manuale.
pub trait EntityDefinition: Component {
    /// Nome leggibile (logging, debug).
    fn name() -> &'static str;

    /// Bundle di componenti identità/dati specifiche (solo marker + componenti
    /// proprie di questa entità, NON `Position`/`EntityColor`/`Health` che sono
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

    /// Salute iniziale. Default 100.
    fn health() -> Health {
        Health::new(100.0)
    }

    /// Statistiche iniziali di movimento e combattimento.
    fn stats() -> Stats {
        Stats::default()
    }

    /// Target di replicazione lightyear di default. Override solo se serve
    /// un target diverso (es. `NetworkTarget::AllExceptSingle(peer)`).
    fn replication_target() -> NetworkTarget {
        NetworkTarget::All
    }
}
