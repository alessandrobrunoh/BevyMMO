//! Concrete creature placeable definitions: player spawn, enemy and boss.

pub mod boss_dragon;
pub mod dummy;
pub mod goblin;
pub mod player_spawn;

use crate::placeables::PlaceableRegistry;

/// Registers every creature kind. Called by
/// `crate::content::placeables::register_all`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    player_spawn::register(registry);
    goblin::register(registry);
    dummy::register(registry);
    boss_dragon::register(registry);
}
