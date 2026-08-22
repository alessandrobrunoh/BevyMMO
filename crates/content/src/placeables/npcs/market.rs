//! Isolated player-market NPCs.

use crate::placeables::PlaceableRegistry;

mod npc_market_1 {
    use crate::placeables::npc;

    #[npc(
        id = "npc_market_1",
        name = "Market 1",
        icon = "🏦",
        asset = "models/npcs/merchant.glb",
        tint = (0.75, 0.6, 0.25),
        interaction = market("market_1"),
    )]
    pub struct Market1;
}

mod npc_market_2 {
    use crate::placeables::npc;

    #[npc(
        id = "npc_market_2",
        name = "Market 2",
        icon = "🏦",
        asset = "models/npcs/merchant.glb",
        tint = (0.75, 0.6, 0.25),
        interaction = market("market_2"),
    )]
    pub struct Market2;
}

pub fn register(registry: &mut PlaceableRegistry) {
    npc_market_1::register(registry);
    npc_market_2::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placeables::{AssetHint, InteractionKind, KindId, NpcPlaceable};
    use bevymmo_gameplay::markets::{MARKET_1_ID, MARKET_2_ID};

    fn assert_market_npc(def: &dyn NpcPlaceable, kind: &str, name: &str, expected_market_id: &str) {
        assert_eq!(def.id().as_str(), kind);
        assert_eq!(def.display_name(), name);
        assert_eq!(def.icon(), "🏦");
        assert!(matches!(
            def.asset_hint(),
            AssetHint::Scene("models/npcs/merchant.glb")
        ));
        assert_eq!(def.defaults().tint, Some([0.75, 0.6, 0.25]));
        match def.interaction() {
            InteractionKind::Market { market_id } => {
                assert_eq!(market_id, expected_market_id);
            }
            other => panic!("expected Market, got {other:?}"),
        }
    }

    #[test]
    fn market_npcs_keep_isolated_halls() {
        let mut registry = PlaceableRegistry::default();
        register(&mut registry);

        let one = registry
            .npcs
            .get(&KindId::new("npc_market_1"))
            .expect("npc_market_1 is registered");
        assert_market_npc(one.as_ref(), "npc_market_1", "Market 1", MARKET_1_ID);

        let two = registry
            .npcs
            .get(&KindId::new("npc_market_2"))
            .expect("npc_market_2 is registered");
        assert_market_npc(two.as_ref(), "npc_market_2", "Market 2", MARKET_2_ID);
    }
}
