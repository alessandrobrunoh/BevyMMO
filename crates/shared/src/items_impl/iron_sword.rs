//! "Spada 1" — the first concrete item.
//!
//! Grants +1000 MaxHealth while equipped, as a permanent stat bonus. This is
//! the reference implementation for an equippable weapon and the test fixture
//! for the inventory/equip pipeline.
//!
//! Also the reference implementation of a spell-granting item built *by
//! hand* instead of through the `#[item(...)]` macro (see `magic_staff.rs`
//! for the macro version): `spell_kit` is just a regular field, built once
//! in `new()` and returned by overriding `Item::spell_kit`. Useful when an
//! item's data can't be expressed as a macro literal (e.g. computed at
//! runtime) — the macro is sugar for the common case, not a requirement.

use std::borrow::Cow;

use crate::items::components::EquipSlot;
use crate::items::definition::{Item, ItemCategory, ItemConfig, ItemRarity};
use crate::items::effects::ItemEffect;
use crate::items::registry::ItemId;
use crate::items::spell_kit::SpellKit;
use crate::spells::registry::SpellId;
use crate::spells_impl::attack::AttackSpell;
use crate::spells_impl::healing_circle::HealingCircleSpell;
use crate::spells_impl::stun_field::StunFieldSpell;
use crate::spells_impl::swift::SwiftSpell;
use crate::stats::events::{ModifierOp, StatField};

/// The iron sword ("Spada 1").
pub struct IronSword {
    config: ItemConfig,
    effects: Vec<ItemEffect>,
    spell_kit: SpellKit,
}

impl IronSword {
    /// Stable id used by the registry, network commands and persistence.
    pub const ID: &'static str = "iron_sword";

    /// Health bonus granted while equipped.
    pub const MAX_HEALTH_BONUS: f32 = 1000.0;

    /// Builds a new instance with the canonical config and effects.
    pub fn new() -> Self {
        Self {
            config: ItemConfig {
                display_name: Cow::Borrowed("Spada 1"),
                description: Cow::Borrowed("Forgiata nei meandri delle fucine dimenticate, questa antica lama emana un'energia vitale ancestrale che rinvigorisce chiunque la impugni in battaglia."),
                category: ItemCategory::Weapon,
                rarity: ItemRarity::Uncommon,
                equippable_into: Some(EquipSlot::Weapon),
                weight: 0.0,
            },
            effects: vec![ItemEffect::StatBonus {
                field: StatField::MaxHealth,
                op: ModifierOp::Add,
                value: Self::MAX_HEALTH_BONUS,
            }],
            // Warrior/vitality flavor: a plain swing or a stunning heavy
            // strike on Q, a charge on W, and — matching the MaxHealth bonus
            // above — a self-sustain burst on E.
            spell_kit: SpellKit::new(
                vec![SpellId::new(AttackSpell::ID), SpellId::new(StunFieldSpell::ID)],
                vec![SpellId::new(SwiftSpell::ID)],
                SpellId::new(HealingCircleSpell::ID),
            ),
        }
    }
}

impl Default for IronSword {
    fn default() -> Self {
        Self::new()
    }
}

impl Item for IronSword {
    fn id(&self) -> ItemId {
        ItemId::new(Self::ID)
    }
    fn config(&self) -> &ItemConfig {
        &self.config
    }
    fn effects(&self) -> &[ItemEffect] {
        &self.effects
    }
    fn spell_kit(&self) -> Option<&SpellKit> {
        Some(&self.spell_kit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_stable() {
        let sword = IronSword::new();
        assert_eq!(sword.id().as_str(), IronSword::ID);
        assert_eq!(IronSword::ID, "iron_sword");
    }

    #[test]
    fn display_name_matches_design() {
        let sword = IronSword::new();
        assert_eq!(sword.display_name(), "Spada 1");
    }

    #[test]
    fn is_equippable_into_weapon_slot() {
        let sword = IronSword::new();
        assert_eq!(sword.config().equippable_into, Some(EquipSlot::Weapon));
    }

    #[test]
    fn grants_plus_1000_max_health_while_equipped() {
        let sword = IronSword::new();
        let effects = sword.effects();
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            ItemEffect::StatBonus { field, op, value } => {
                assert_eq!(*field, StatField::MaxHealth);
                assert_eq!(*op, ModifierOp::Add);
                assert_eq!(*value, IronSword::MAX_HEALTH_BONUS);
                assert_eq!(*value, 1000.0);
            }
            other => panic!("expected StatBonus, got {other:?}"),
        }
        assert!(effects[0].is_passive_while_equipped());
    }

    #[test]
    fn has_no_equip_requirements() {
        let sword = IronSword::new();
        assert!(sword.equip_requirements().is_empty());
    }

    #[test]
    fn grants_two_q_options_one_w_one_e() {
        let sword = IronSword::new();
        let kit = sword.spell_kit().expect("iron_sword must grant a spell kit");

        assert_eq!(
            kit.candidates_for(crate::spells::components::HotbarSlot::Q),
            &[SpellId::new(AttackSpell::ID), SpellId::new(StunFieldSpell::ID)]
        );
        assert_eq!(
            kit.candidates_for(crate::spells::components::HotbarSlot::W),
            &[SpellId::new(SwiftSpell::ID)]
        );
        assert_eq!(
            kit.candidates_for(crate::spells::components::HotbarSlot::E),
            &[SpellId::new(HealingCircleSpell::ID)]
        );
    }
}
