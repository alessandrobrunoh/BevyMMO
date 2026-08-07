//! Presentation-side visuals for the dragon boss ability kit.
//!
//! Each submodule hosts the client-only `visual` for one dragon ability.
//! `cinder_storm` is intentionally not declared here: its definition (and
//! therefore its visual that needs the spell's constants) still lives in the
//! binary because it depends on server-only target selection helpers.

pub mod cataclysm;
pub mod dragon_claw;
pub mod molten_eruption;
pub mod searing_breath;
pub mod tail_sweep;
pub mod wing_buffet;
