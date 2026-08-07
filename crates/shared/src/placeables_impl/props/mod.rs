//! Concrete prop definitions for the placeable catalog.

pub mod bush_01;
pub mod crate_01;
pub mod cube;
pub mod fence_01;
pub mod house_simple;
pub mod lamp_01;
pub mod rock_01;
pub mod rock_02;
pub mod statue_01;
pub mod tree_oak;

use crate::placeables::PlaceableRegistry;

/// Registers every prop kind. Called by
/// `crate::placeables_impl::register_default_placeables`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    bush_01::register(registry);
    crate_01::register(registry);
    cube::register(registry);
    fence_01::register(registry);
    house_simple::register(registry);
    lamp_01::register(registry);
    rock_01::register(registry);
    rock_02::register(registry);
    statue_01::register(registry);
    tree_oak::register(registry);
}
