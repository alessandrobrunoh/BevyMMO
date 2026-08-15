//! "Traveler's Cape" — reference `Cape` slot item.
//!
//! Grants a small movement speed bonus while equipped.

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::stats::events::{ModifierOp, StatField};

pub struct TravelersCape {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl TravelersCape {
    pub const ID: &'static str = "travelers_cape";
    pub const SPEED_BONUS: f32 = 0.03;

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Traveler's Cape"),
                description: Cow::Borrowed(
                    "A wind-worn cape that lightens every stride, favored by messengers and scouts.",
                ),
                category: ItemCategory::Accessory,
                rarity: ItemRarity::Common,
                equippable_into: Some(EquipSlot::Cape),
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

impl Default for TravelersCape {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for TravelersCape {
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
    fn is_equippable_into_cape_slot() {
        let item = TravelersCape::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Cape));
    }
}
