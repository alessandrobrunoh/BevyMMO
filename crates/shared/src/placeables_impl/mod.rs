//! Concrete placeable implementations.
//!
//! Each submodule is a self-contained definition registered at startup
//! via [`register_default_placeables`]. This mirrors the spell framework's
//! `spells_impl` module.

pub mod props;

use crate::placeables::PlaceableRegistry;

/// Registers every placeable kind available to the current game build.
///
/// Called at startup by the application root (game binary). Both the editor
/// and the server use the same registry, so the palette and the spawn
/// machinery always agree on what kinds exist.
pub fn register_default_placeables(registry: &mut PlaceableRegistry) {
    props::register_all(registry);
}
