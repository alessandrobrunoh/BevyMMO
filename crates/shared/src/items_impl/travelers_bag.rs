//! "Traveler's Bag" — reference `Bag` slot item.
//!
//! Purely cosmetic for now: it occupies the `Bag` slot but grants no stat
//! bonus. Reserved hook for a future inventory-capacity system.

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;

pub struct TravelersBag {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl TravelersBag {
    pub const ID: &'static str = "travelers_bag";

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Traveler's Bag"),
                description: Cow::Borrowed(
                    "A sturdy canvas pack with more pockets than it has any right to.",
                ),
                category: ItemCategory::Accessory,
                rarity: ItemRarity::Common,
                equippable_into: Some(EquipSlot::Bag),
                weight: 0.0,
            },
            effects: Vec::new(),
        }
    }
}

impl Default for TravelersBag {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for TravelersBag {
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
    fn is_equippable_into_bag_slot() {
        let item = TravelersBag::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Bag));
    }
}
