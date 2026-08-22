//! Greeter NPC with a dialogue interaction.

use crate::placeables::npc;

#[npc(
    id = "npc_greeter",
    name = "Greeter",
    icon = "👋",
    interaction = dialogue("greeting"),
)]
pub struct Greeter;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placeables::{
        AssetHint, InteractionKind, NpcPlaceable, PlaceableDefinition, PlaceableRegistry,
    };

    #[test]
    fn greeter_keeps_dialogue_kind_and_placeholder() {
        let def = Greeter;
        assert_eq!(def.id().as_str(), "npc_greeter");
        assert_eq!(Greeter::ID, "npc_greeter");
        assert_eq!(def.display_name(), "Greeter");
        assert_eq!(def.icon(), "👋");
        assert!(matches!(def.asset_hint(), AssetHint::Placeholder));
        assert_eq!(def.defaults().tint, None);
        match def.interaction() {
            InteractionKind::Dialogue { dialogue_tree_id } => {
                assert_eq!(dialogue_tree_id, "greeting");
            }
            other => panic!("expected Dialogue, got {other:?}"),
        }

        let mut registry = PlaceableRegistry::default();
        register(&mut registry);
        assert!(registry.npcs.contains_key(&def.id()));
    }
}
