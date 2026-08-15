//! Concrete creature placeable definitions: player spawn, enemy and boss.

pub mod boss_dragon;
pub mod goblin;
pub mod player_spawn;

use crate::placeables::PlaceableRegistry;

/// Registers every creature kind. Called by
/// `crate::placeables_impl::register_default_placeables`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    player_spawn::register(registry);
    goblin::register(registry);
    boss_dragon::register(registry);
}
