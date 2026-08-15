//! "Quick Flask" — reference `Potion` slot item.
//!
//! A primed flask kept in the quick-access potion slot. Grants a small mana
//! regeneration bonus while equipped; drinking/consuming it is a future
//! extension (`ItemEffect::InstantHeal` on a `UseItemCommand`).

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::stats::events::{ModifierOp, StatField};

pub struct QuickFlask {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
}

impl QuickFlask {
    pub const ID: &'static str = "quick_flask";
    pub const MANA_REGEN_BONUS: f32 = 5.0;

    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Quick Flask"),
                description: Cow::Borrowed(
                    "A stoppered flask kept within reach, its contents humming with restorative energy.",
                ),
                category: ItemCategory::Consumable,
                rarity: ItemRarity::Common,
                equippable_into: Some(EquipSlot::Potion),
                weight: 0.0,
            },
            effects: vec![ItemEffect::StatBonus {
                field: StatField::ManaRegeneration,
                op: ModifierOp::Add,
                value: Self::MANA_REGEN_BONUS,
            }],
        }
    }
}

impl Default for QuickFlask {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for QuickFlask {
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
    fn is_equippable_into_potion_slot() {
        let item = QuickFlask::new();
        assert_eq!(item.config().equippable_into, Some(EquipSlot::Potion));
    }
}
