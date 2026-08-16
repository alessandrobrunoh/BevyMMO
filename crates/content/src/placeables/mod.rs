//! Concrete placeable content grouped by world category.

pub mod creatures;
pub mod interactables;
pub mod npcs;
pub mod props;
pub mod resources;
pub mod triggers;

use crate::placeables::PlaceableRegistry;

/// Registers every placeable kind shipped by this game build.
pub fn register_all(registry: &mut PlaceableRegistry) {
    props::register_all(registry);
    creatures::register_all(registry);
    npcs::register_all(registry);
    triggers::register_all(registry);
    resources::register_all(registry);
    interactables::register_all(registry);
}
