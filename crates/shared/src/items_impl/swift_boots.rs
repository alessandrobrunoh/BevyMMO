//! "Swift Boots" — reference `Shoes` slot item.
//!
//! Grants a small movement speed bonus while equipped.

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::stats::events::{ModifierOp, StatField};

pub struct SwiftBoots {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl SwiftBoots {
    pub const ID: &'static str = "swift_boots";
    pub const SPEED_BONUS: f32 = 0.05;

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Swift Boots"),
                description: Cow::Borrowed(
                    "Soft-soled boots enchanted to lighten the wearer's steps.",
                ),
                category: ItemCategory::Armor,
                rarity: ItemRarity::Uncommon,
                equippable_into: Some(EquipSlot::Shoes),
                weight: 0.0,
            },
            effects: vec![ItemEffect::StatBonus {
                field: StatField::Speed,
                op: ModifierOp::Add,
                value: Self::SPEED_BONUS,
            }],
        }
    }
}

impl Default for SwiftBoots {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for SwiftBoots {
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
    fn is_equippable_into_shoes_slot() {
        let item = SwiftBoots::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Shoes));
    }
}
