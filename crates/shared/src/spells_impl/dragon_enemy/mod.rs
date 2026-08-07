//! Vermithrax, the Ashen Drake — boss ability kit.
//!
//! All dragon abilities live under this parent module to keep the encounter's
//! spells grouped. Each ability gets its own submodule
//! (`<name>/{definition.rs, mod.rs}`). Shared visual color constants
//! keep the kit visually consistent.
//!
//! NOTE: `cinder_storm` is intentionally NOT declared here. Its definition
//! depends on `bevymmo_server::gameplay::entity::boss::target_select`, and
//! `bevymmo_shared` cannot depend on the server crate. `cinder_storm` stays
//! in the binary for now (see `bins/game/src/spells/dragon_enemy/cinder_storm`).

pub mod cataclysm;
pub mod dragon_claw;
pub mod molten_eruption;
pub mod searing_breath;
pub mod tail_sweep;
pub mod wing_buffet;

use bevy::color::LinearRgba;

/// Warning red for ground telegraphs (sRGB `(0.95, 0.10, 0.10)`).
pub const ASH_RED: LinearRgba = LinearRgba::rgb(0.60, 0.05, 0.05);
/// Fire orange for impacts (sRGB `(1.00, 0.45, 0.05)`).
pub const FIRE_ORANGE: LinearRgba = LinearRgba::rgb(0.95, 0.40, 0.05);
/// Hot ember yellow for cores and sparks (sRGB `(1.00, 0.85, 0.25)`).
pub const EMBER_YELLOW: LinearRgba = LinearRgba::rgb(1.00, 0.70, 0.20);
/// Residual smoke after fire fades (sRGB `(0.25, 0.20, 0.18)`).
pub const SMOKE_GRAY: LinearRgba = LinearRgba::rgb(0.05, 0.04, 0.04);
/// Dusty tan for wing-buffet shockwaves (sRGB `(0.80, 0.70, 0.55)`).
pub const DUST_TAN: LinearRgba = LinearRgba::rgb(0.15, 0.13, 0.10);
