//! Concrete trigger zone definitions.
//!
//! Each trigger kind is a self-contained definition registered at startup
//! via [`register_all`].

pub mod pvp_zone;
pub mod safe_zone;
pub mod teleport;

use crate::placeables::PlaceableRegistry;

/// Registers every trigger kind. Called by
/// `crate::content::placeables::register_all`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    pvp_zone::register(registry);
    safe_zone::register(registry);
    teleport::register(registry);
}
