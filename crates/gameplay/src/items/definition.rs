//! Static metadata and the `Item` trait contract.
//!
//! Mirrors `crate::spells::context::Spell`: the trait is the contract that
//! every concrete item implements; static metadata lives in [`ItemConfig`].
//! Concrete implementations live in `crate::content::items`.

use std::borrow::Cow;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::abilities::{AbilityBlueprint, AbilityLoadout, BaseAbility, RuneProfile};

use super::components::EquipSlot;
use super::effects::ItemEffect;
use super::registry::ItemId;
use super::spell_kit::SpellKit;
use super::weapon_family::WeaponFamilyId;

/// Narrative category, used by the inventory UI (filtering / icons) and by
/// equip validation (only `Weapon` items can go into the weapon slot, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemCategory {
    Weapon,
    Armor,
    Consumable,
    Material,
    Quest,
    /// Worn utility items that are neither weapon nor armor (bag, cape, mount).
    Accessory,
}

/// Rarity, purely cosmetic for now (drives slot border color in the UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// Static metadata shared by every item.
///
/// Stack size is not an item property: [`super::components::Inventory::MAX_STACK`]
/// is the bag cap, and only [`ItemCategory::Material`] stacks.
#[derive(Debug, Clone)]
pub struct ItemConfig {
    /// Player-facing name shown in the inventory and detail cards.
    pub display_name: Cow<'static, str>,
    /// Longer flavor / tooltip text shown in the item detail card.
    pub description: Cow<'static, str>,
    /// Filtering / icon category.
    pub category: ItemCategory,
    /// Cosmetic rarity driving the slot border color.
    pub rarity: ItemRarity,
    /// Slot this item can be equipped into (`None` = inventory-only item).
    pub equippable_into: Option<EquipSlot>,
    /// Reserved for a future encumbrance system. 0 for now.
    pub weight: f32,
    /// Whether this item can change owner via the player market.
    ///
    /// Defaults to `true` in `#[item(...)]`. Soulbound / quest items set
    /// `tradable = false` and never appear in a hall's sell list.
    pub tradable: bool,
}

/// Contract every concrete item implements.
///
/// The trait is deliberately small: identity, static metadata and the list of
/// gameplay effects. Anything that mutates the world (applying a buff,
/// spawning a projectile) is server-side and reads from `effects()`.
///
/// # Example
/// ```ignore
/// use std::sync::Arc;
/// use bevymmo_shared::items::{Item, ItemRegistry};
///
/// let mut registry = ItemRegistry::default();
/// registry.register(Arc::new(my_item));
/// ```
pub trait Item: Send + Sync + 'static {
    /// Stable unique id of this item type.
    fn id(&self) -> ItemId;

    /// Static metadata (name, description, category, ...).
    fn config(&self) -> &ItemConfig;

    /// Player-facing display name. Defaults to `config().display_name`.
    fn display_name(&self) -> &str {
        &self.config().display_name
    }

    /// Whether this item can change owner via the player market.
    fn tradable(&self) -> bool {
        self.config().tradable
    }

    /// Effects applied while equipped (StatBonus), or on use for consumables.
    fn effects(&self) -> &[ItemEffect];

    /// Equip requirements (level, class, ...). Empty slice = always
    /// equippable. The server reads this when validating an equip command.
    fn equip_requirements(&self) -> &[EquipRequirement] {
        &[]
    }

    /// Spells this item makes available on Primary/Secondary/Ultimate while
    /// equipped.
    ///
    /// `None` (the default) means the item grants no spells at all — most
    /// armor/accessory items only contribute [`effects`](Item::effects) and
    /// never override this. Items that *do* grant spells implement it via
    /// the `#[item(..., spells(primary = [...], secondary = [...], ultimate = ...))]`
    /// macro, which also enforces the Primary(1+)/Secondary(1+)/Ultimate(1)
    /// shape at compile time; see `bevymmo_server::items::available_spells`
    /// for how kits from every equipped item are unioned into the player's
    /// selectable spell pool.
    fn spell_kit(&self) -> Option<&SpellKit> {
        None
    }

    /// Shared weapon category. `None` for non-weapon items or items that do
    /// not participate in the weapon-family system yet.
    fn weapon_family(&self) -> Option<WeaponFamilyId> {
        None
    }

    /// Abilità offerte da questo item. Lo stesso loadout è usato da armi e
    /// armature; `None` indica un item senza abilità proprie.
    fn ability_loadout(&self) -> Option<&AbilityLoadout> {
        None
    }

    /// Applies item-specific execution rules to a derived ability blueprint.
    /// Root Words and Ancient Words will join this pipeline later.
    fn transform_ability_blueprint(&self, _blueprint: &mut AbilityBlueprint) {}

    fn ability_blueprint(&self, ability: &dyn BaseAbility) -> AbilityBlueprint {
        let mut blueprint = ability.blueprint();
        self.transform_ability_blueprint(&mut blueprint);
        blueprint
    }

    /// Capacità Runica / Stabilità / Affinità — quanto può reggere inciso.
    fn rune_profile(&self) -> Option<&RuneProfile> {
        None
    }
}

/// Dyn-compatible alias used when storing items inside the registry.
///
/// Defined as a trait alias for ergonomics; equivalent to `Arc<dyn Item>`.
pub type ArcItem = Arc<dyn Item>;

/// Reserved hook for future equip rules (level, class, ...).
///
/// The variant set is intentionally minimal: extend it only when concrete
/// requirements are needed, so existing serialized data stays compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipRequirement {
    /// Minimum player level to equip the item.
    MinLevel { value: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy {
        config: ItemConfig,
    }

    impl Item for Dummy {
        fn id(&self) -> ItemId {
            ItemId::new("dummy")
        }
        fn config(&self) -> &ItemConfig {
            &self.config
        }
        fn effects(&self) -> &[ItemEffect] {
            &[]
        }
    }

    fn sample_config() -> ItemConfig {
        ItemConfig {
            display_name: Cow::Borrowed("Dummy"),
            description: Cow::Borrowed(""),
            category: ItemCategory::Weapon,
            rarity: ItemRarity::Common,
            equippable_into: Some(EquipSlot::Weapon),
            weight: 0.0,
            tradable: true,
        }
    }

    #[test]
    fn display_name_defaults_to_config_value() {
        let item = Dummy {
            config: sample_config(),
        };
        assert_eq!(item.display_name(), "Dummy");
    }

    #[test]
    fn tradable_defaults_to_config_value() {
        let item = Dummy {
            config: sample_config(),
        };
        assert!(item.tradable());

        let bound = Dummy {
            config: ItemConfig {
                tradable: false,
                ..sample_config()
            },
        };
        assert!(!bound.tradable());
    }

    #[test]
    fn equip_requirements_defaults_to_empty() {
        let item = Dummy {
            config: sample_config(),
        };
        assert!(item.equip_requirements().is_empty());
    }
}
