//! Static metadata and the `Item` trait contract.
//!
//! Mirrors `crate::spells::context::Spell`: the trait is the contract that
//! every concrete item implements; static metadata lives in [`ItemConfig`].
//! Concrete implementations live in `crate::content::items`.

use std::borrow::Cow;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::abilities::{RuneProfile, WeaponAbilities};

use super::components::EquipSlot;
use super::effects::ItemEffect;
use super::registry::ItemId;
use super::spell_kit::SpellKit;

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
/// `max_stack` is intentionally absent: 1 item = 1 slot (decision #1 of
/// `plans/inventory-system.md`). It can be reintroduced later without breaking
/// saved data because the inventory layout is a fixed-size array of ids.
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

    /// Effects applied while equipped (StatBonus), or on use for consumables.
    fn effects(&self) -> &[ItemEffect];

    /// Equip requirements (level, class, ...). Empty slice = always
    /// equippable. The server reads this when validating an equip command.
    fn equip_requirements(&self) -> &[EquipRequirement] {
        &[]
    }

    /// Spells this item makes available on the Q/W/E hotbar while equipped.
    ///
    /// `None` (the default) means the item grants no spells at all — most
    /// armor/accessory items only contribute [`effects`](Item::effects) and
    /// never override this. Items that *do* grant spells implement it via
    /// the `#[item(..., spells(q = [...], w = [...], e = ...))]` macro,
    /// which also enforces the Q(1+)/W(1+)/E(1) shape at compile time; see
    /// `bevymmo_server::items::available_spells` for how kits from every
    /// equipped item are unioned into the player's selectable spell pool.
    fn spell_kit(&self) -> Option<&SpellKit> {
        None
    }

    /// I tre gesti fissi (Primary/Secondary/Ultimate) di questa variante
    /// d'arma — sistema "Eidolon": il gesto è dell'arma, cosa manifesta
    /// dipende dall'Incisione del giocatore su quell'esemplare (vedi
    /// `crate::abilities`). `None` per item senza gesti propri.
    ///
    /// Coesiste con [`spell_kit`](Item::spell_kit): un'arma usa l'uno O
    /// l'altro modello, non entrambi. Nuove armi dovrebbero preferire
    /// questo; `spell_kit` resta per gli item già esistenti.
    fn weapon_abilities(&self) -> Option<&WeaponAbilities> {
        None
    }

    /// Capacità Runica / Stabilità / Affinità — quanto e cosa questa arma
    /// può reggere inciso. `None` se `weapon_abilities()` è `None`.
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
    fn equip_requirements_defaults_to_empty() {
        let item = Dummy {
            config: sample_config(),
        };
        assert!(item.equip_requirements().is_empty());
    }
}
