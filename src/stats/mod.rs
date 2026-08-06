//! Modulo delle statistiche di gioco.
//!
//! Possiede i componenti runtime ([`components`]), gli eventi
//! ([`events`]), le formule ([`formulas`]), i modifier ([`modifiers`]),
//! i sistemi ([`systems`]) e i profili default ([`defaults`]).
//!
//! Il plugin registra tutto quanto serve per applicare danno, cura e
//! modifier in modo server-authoritative.

pub mod components;
pub mod defaults;
pub mod events;
pub mod formulas;
pub mod modifiers;
pub mod systems;

pub use plugin::StatsPlugin;

mod plugin;
