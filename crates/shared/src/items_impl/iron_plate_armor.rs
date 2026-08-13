//! "Iron Plate Armor" — reference `Armor` slot item.
//!
//! Grants both extra Max Health and Armor while equipped.

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::stats::events::{ModifierOp, StatField};

pub struct IronPlateArmor {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl IronPlateArmor {
    pub const ID: &'static str = "iron_plate_armor";
    pub const MAX_HEALTH_BONUS: f32 = 300.0;
    pub const ARMOR_BONUS: f32 = 30.0;

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Iron Plate Armor"),
                description: Cow::Borrowed(
                    "Heavy riveted plates that shrug off blows at the cost of a little grace.",
                ),
                category: ItemCategory::Armor,
                rarity: ItemRarity::Uncommon,
                equippable_into: Some(EquipSlot::Armor),
                weight: 0.0,
            },
            effects: vec![
                ItemEffect::StatBonus {
                    field: StatField::MaxHealth,
                    op: ModifierOp::Add,
                    value: Self::MAX_HEALTH_BONUS,
                },
                ItemEffect::StatBonus {
                    field: StatField::Armor,
                    op: ModifierOp::Add,
                    value: Self::ARMOR_BONUS,
                },
            ],
        }
    }
}

impl Default for IronPlateArmor {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for IronPlateArmor {
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
    fn is_equippable_into_armor_slot() {
        let item = IronPlateArmor::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Armor));
    }

    #[test]
    fn grants_two_stat_bonuses() {
        let item = IronPlateArmor::new();
        assert_eq!(item.effects().len(), 2);
    }
}
