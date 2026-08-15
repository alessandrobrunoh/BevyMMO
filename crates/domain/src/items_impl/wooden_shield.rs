//! "Wooden Shield" — reference `Offhand` slot item.
//!
//! Grants +35 Armor while equipped.

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::stats::events::{ModifierOp, StatField};

pub struct WoodenShield {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl WoodenShield {
    pub const ID: &'static str = "wooden_shield";
    pub const ARMOR_BONUS: f32 = 35.0;

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Wooden Shield"),
                description: Cow::Borrowed(
                    "Banded oak reinforced with an iron rim. Simple, reliable, always in the way of harm.",
                ),
                category: ItemCategory::Armor,
                rarity: ItemRarity::Common,
                equippable_into: Some(EquipSlot::Offhand),
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

impl Default for WoodenShield {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for WoodenShield {
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
    fn is_equippable_into_offhand_slot() {
        let item = WoodenShield::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Offhand));
    }
}
