//! Weapon crafter NPC. Lists catalogue weapons that declare a recipe.

use std::sync::Arc;

use crate::items::definition::ItemCategory;
use crate::placeables::{
    AssetHint, InteractionKind, KindId, NpcPlaceable, PlaceableDefaults, PlaceableDefinition,
    PlaceableRegistry,
};
use crate::world::TransformData;

pub struct WeaponCrafterDefinition;

impl PlaceableDefinition for WeaponCrafterDefinition {
    fn id(&self) -> KindId {
        KindId::new("npc_weapon_crafter")
    }
    fn display_name(&self) -> &'static str {
        "Fabbro"
    }
    fn icon(&self) -> &'static str {
        "🔨"
    }
    fn asset_hint(&self) -> AssetHint {
        AssetHint::Scene("models/npcs/merchant.glb")
    }
    fn defaults(&self) -> PlaceableDefaults {
        PlaceableDefaults {
            transform: TransformData {
                translation: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            tint: Some([0.85, 0.55, 0.25]),
            collision: None,
            blocks_movement: false,
        }
    }
}

impl NpcPlaceable for WeaponCrafterDefinition {
    fn interaction(&self) -> InteractionKind {
        InteractionKind::Craft {
            category: ItemCategory::Weapon,
        }
    }
}

pub fn register(registry: &mut PlaceableRegistry) {
    registry.register_npc(Arc::new(WeaponCrafterDefinition));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placeables::PlaceableDefinition;

    #[test]
    fn weapon_crafter_offers_weapon_recipes() {
        let def = WeaponCrafterDefinition;
        assert_eq!(def.id().as_str(), "npc_weapon_crafter");
        assert_eq!(
            def.interaction(),
            InteractionKind::Craft {
                category: ItemCategory::Weapon
            }
        );
    }
}
