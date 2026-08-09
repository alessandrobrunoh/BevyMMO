//! Concrete prop definitions for the placeable catalog.

pub mod boulder_large;
pub mod bush_01;
pub mod crate_01;
pub mod cube;
pub mod fence_01;
pub mod house_simple;
pub mod lamp_01;
pub mod pebbles;
pub mod rock;
pub mod rock_01;
pub mod rock_02;
pub mod rock_enormous;
pub mod rock_large;
pub mod rock_medium;
pub mod rock_mini;
pub mod rock_small;
pub mod statue_01;
pub mod tree_birch_medium;
pub mod tree_oak;
pub mod tree_oak_large;
pub mod tree_oak_medium;
pub mod tree_oak_small;
pub mod tree_pine_large;
pub mod tree_pine_medium;
pub mod tree_pine_small;
pub mod yggdrasil;

use crate::placeables::PlaceableRegistry;

/// Registers every prop kind. Called by
/// `crate::placeables_impl::register_default_placeables`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    boulder_large::register(registry);
    bush_01::register(registry);
    crate_01::register(registry);
    cube::register(registry);
    fence_01::register(registry);
    house_simple::register(registry);
    lamp_01::register(registry);
    pebbles::register(registry);
    rock::register(registry);
    rock_01::register(registry);
    rock_02::register(registry);
    rock_enormous::register(registry);
    rock_large::register(registry);
    rock_medium::register(registry);
    rock_mini::register(registry);
    rock_small::register(registry);
    statue_01::register(registry);
    tree_birch_medium::register(registry);
    tree_oak::register(registry);
    tree_oak_large::register(registry);
    tree_oak_medium::register(registry);
    tree_oak_small::register(registry);
    tree_pine_large::register(registry);
    tree_pine_medium::register(registry);
    tree_pine_small::register(registry);
    yggdrasil::register(registry);
}
