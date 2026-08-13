//! "Swift Steed" — reference `Mount` slot item.
//!
//! Grants a large movement speed bonus while equipped.

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::stats::events::{ModifierOp, StatField};

pub struct SwiftSteed {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl SwiftSteed {
    pub const ID: &'static str = "swift_steed";
    pub const SPEED_BONUS: f32 = 0.15;

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Swift Steed"),
                description: Cow::Borrowed(
                    "A loyal steed, saddled and ready. Crosses the realm far faster than any pair of boots.",
                ),
                category: ItemCategory::Accessory,
                rarity: ItemRarity::Rare,
                equippable_into: Some(EquipSlot::Mount),
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

impl Default for SwiftSteed {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for SwiftSteed {
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
    fn is_equippable_into_mount_slot() {
        let item = SwiftSteed::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Mount));
    }
}
