//! "Leather Helmet" — reference `Helmet` slot item.
//!
//! Grants +40 Armor while equipped.

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::stats::events::{ModifierOp, StatField};

pub struct LeatherHelmet {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl LeatherHelmet {
    pub const ID: &'static str = "leather_helmet";
    pub const ARMOR_BONUS: f32 = 40.0;

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Leather Helmet"),
                description: Cow::Borrowed(
                    "Boiled leather stitched over a light iron frame. Sturdy enough to turn a blade.",
                ),
                category: ItemCategory::Armor,
                rarity: ItemRarity::Common,
                equippable_into: Some(EquipSlot::Helmet),
                weight: 0.0,
            },
            effects: vec![ItemEffect::StatBonus {
                field: StatField::Armor,
                op: ModifierOp::Add,
                value: Self::ARMOR_BONUS,
            }],
        }
    }
}

impl Default for LeatherHelmet {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for LeatherHelmet {
    fn id(&self) -> ItemId {
        ItemId::new(Self::ID)
    }
    fn config(&self) -> &ItemConfig {
        &self.config
    }
    fn effects(&self) -> &[ItemEffect] {
        &self.effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_equippable_into_helmet_slot() {
        let item = LeatherHelmet::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Helmet));
    }

    #[test]
    fn grants_armor_bonus() {
        let item = LeatherHelmet::new();
        assert_eq!(item.effects().len(), 1);
    }
}
