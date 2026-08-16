//! Concrete resource node definitions.
//!
//! Each resource kind is a self-contained definition registered at startup
//! via [`register_all`].

pub mod copper_vein;

use crate::placeables::PlaceableRegistry;

/// Registers every resource node kind. Called by
/// `crate::content::placeables::register_all`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    copper_vein::register(registry);
}
