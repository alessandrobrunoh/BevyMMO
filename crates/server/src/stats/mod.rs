//! Server-authoritative stats application.
//!
//! Registers Bevy messages and runs the systems that apply damage, healing,
//! buffs/debuffs and modifier expiry. Raw stat data lives in `bevymmo_shared`.

pub mod plugin;
pub mod systems;

pub use plugin::StatsPlugin;
