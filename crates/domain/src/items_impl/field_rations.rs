//! "Field Rations" — reference `Food` slot item.
//!
//! A well-stocked ration pouch. Grants extra Max Health while equipped
//! (the wearer is simply better fed).

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::stats::events::{ModifierOp, StatField};

pub struct FieldRations {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl FieldRations {
    pub const ID: &'static str = "field_rations";
    pub const MAX_HEALTH_BONUS: f32 = 100.0;

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Field Rations"),
                description: Cow::Borrowed(
                    "Dried meat, hardtack and salt, packed for the road. A well-fed adventurer is a hardy one.",
                ),
                category: ItemCategory::Consumable,
                rarity: ItemRarity::Common,
                equippable_into: Some(EquipSlot::Food),
                weight: 0.0,
            },
            effects: vec![ItemEffect::StatBonus {
                field: StatField::MaxHealth,
                op: ModifierOp::Add,
                value: Self::MAX_HEALTH_BONUS,
            }],
        }
    }
}

impl Default for FieldRations {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for FieldRations {
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
    fn is_equippable_into_food_slot() {
        let item = FieldRations::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Food));
    }
}
