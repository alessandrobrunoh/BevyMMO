//! General-purpose merchant NPC with a shop interaction.

use crate::placeables::npc;

#[npc(
    id = "npc_merchant",
    name = "Merchant",
    icon = "🧑‍💼",
    asset = "models/npcs/merchant.glb",
    tint = (0.6, 0.5, 0.9),
    interaction = shop("shop_general"),
)]
pub struct Merchant;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placeables::{
        AssetHint, InteractionKind, NpcPlaceable, PlaceableDefinition, PlaceableRegistry,
    };

    #[test]
    fn merchant_keeps_shop_kind_and_inventory() {
        let def = Merchant;
        assert_eq!(def.id().as_str(), "npc_merchant");
        assert_eq!(Merchant::ID, "npc_merchant");
        assert_eq!(def.display_name(), "Merchant");
        assert_eq!(def.icon(), "🧑‍💼");
        assert!(matches!(
            def.asset_hint(),
            AssetHint::Scene("models/npcs/merchant.glb")
        ));
        assert_eq!(def.defaults().tint, Some([0.6, 0.5, 0.9]));
        match def.interaction() {
            InteractionKind::Shop { inventory_id } => {
                assert_eq!(inventory_id, "shop_general");
            }
            other => panic!("expected Shop, got {other:?}"),
        }

        let mut registry = PlaceableRegistry::default();
        register(&mut registry);
        assert!(registry.npcs.contains_key(&def.id()));
    }
}
