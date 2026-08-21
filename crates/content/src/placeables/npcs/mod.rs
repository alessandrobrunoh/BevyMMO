//! Concrete NPC definitions.
//!
//! Populated by the catalog-extensions agent. Each NPC kind is a self-contained
//! definition registered at startup via [`register_all`].

pub mod crafter;
pub mod greeter;
pub mod market;
pub mod merchant;

use crate::placeables::PlaceableRegistry;

/// Registers every NPC kind. Called by
/// `crate::content::placeables::register_all`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    greeter::register(registry);
    merchant::register(registry);
    market::register(registry);
    crafter::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placeables::{InteractionKind, KindId};
    use bevymmo_gameplay::markets::{MARKET_1_ID, MARKET_2_ID};

    #[test]
    fn catalog_npcs_keep_kind_ids_and_interactions() {
        let mut registry = PlaceableRegistry::default();
        register_all(&mut registry);

        let merchant = registry
            .npcs
            .get(&KindId::new("npc_merchant"))
            .expect("npc_merchant is registered");
        match merchant.interaction() {
            InteractionKind::Shop { inventory_id } => {
                assert_eq!(inventory_id, "shop_general");
            }
            other => panic!("expected Shop, got {other:?}"),
        }

        let greeter = registry
            .npcs
            .get(&KindId::new("npc_greeter"))
            .expect("npc_greeter is registered");
        match greeter.interaction() {
            InteractionKind::Dialogue { dialogue_tree_id } => {
                assert_eq!(dialogue_tree_id, "greeting");
            }
            other => panic!("expected Dialogue, got {other:?}"),
        }

        let market_1 = registry
            .npcs
            .get(&KindId::new("npc_market_1"))
            .expect("npc_market_1 is registered");
        match market_1.interaction() {
            InteractionKind::Market { market_id } => assert_eq!(market_id, MARKET_1_ID),
            other => panic!("expected Market, got {other:?}"),
        }

        let market_2 = registry
            .npcs
            .get(&KindId::new("npc_market_2"))
            .expect("npc_market_2 is registered");
        match market_2.interaction() {
            InteractionKind::Market { market_id } => assert_eq!(market_id, MARKET_2_ID),
            other => panic!("expected Market, got {other:?}"),
        }

        assert!(registry.npcs.len() >= 4);
    }
}
