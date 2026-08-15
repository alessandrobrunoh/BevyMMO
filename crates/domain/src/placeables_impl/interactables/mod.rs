//! Concrete interactable definitions (doors, chests, levers).
//!
//! Each interactable kind is a self-contained definition registered at
//! startup via [`register_all`].

pub mod treasure_chest;
pub mod wooden_door;

use crate::placeables::PlaceableRegistry;

/// Registers every interactable kind. Called by
/// `crate::placeables_impl::register_default_placeables`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    treasure_chest::register(registry);
    wooden_door::register(registry);
}
