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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilityId;
    use crate::ability_definitions::cleave::Cleave;
    use crate::placeables::KindId;

    #[test]
    fn goblin_kind_id_resolves_the_cleave_kit() {
        let mut registry = PlaceableRegistry::default();
        register_all(&mut registry);
        let definition = registry
            .enemies
            .get(&KindId::new("mob_goblin"))
            .expect("goblin is registered");
        let config = definition.enemy_config();
        assert_eq!(config.abilities[0].ability_id, AbilityId::new(Cleave::ID));
        assert_eq!(config.aggro, 8.0);
        assert_eq!(config.leash_aggro, 20.0);
    }
}
