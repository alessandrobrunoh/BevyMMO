//! Inscription domain model — RootWord-based inscription system.
//!
//! ## New Model (RootWord-based)
//!
//! The new model uses [`RootWordId`] as the primary identifier, with optional
//! secondary words for modification. This supports:
//!
//! - [`WeaponInscription`] — one item-level root word plus slot secondary words
//! - [`AbilityInscription`] — ability-level secondary words
//! - [`ItemInscription`] — enum dispatching to specific inscription types
//!

//! ## Architecture
//!
//! ```text
//! ItemInstance
//! ├── Weapon(WeaponInscription)  (RootWord + secondaries)
//! └── Armor(ArmorInscription)    (RootWord + secondaries)
//! ```

use serde::{Deserialize, Serialize};

use super::ancient_word::AncientWordId;
use super::root_word::RootWordId;
use super::slot::AbilitySlot;

// ══════════════════════════════════════════════════════════════════
// NEW ROOTWORD-BASED INSCRIPTION MODEL
// ══════════════════════════════════════════════════════════════════

/// Secondary word that modifies a Root Word's behavior.
///
/// Unlike the primary [`RootWordId`], secondary words are optional and
/// provide additive or transformative effects rather than defining core identity.
/// Note: Does not derive `Eq`/`Hash` because `f32` (intensity) does not implement them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecondaryWord {
    /// Secondary words belong to the universal Ancient Word vocabulary.
    pub word_id: AncientWordId,
    /// Optional intensity retained for the migration window. Final content
    /// should normally encode trade-offs in the word definition itself.
    #[serde(default = "default_secondary_intensity")]
    pub intensity: f32,
}

fn default_secondary_intensity() -> f32 {
    0.5
}

impl SecondaryWord {
    pub fn new(word_id: AncientWordId) -> Self {
        Self {
            word_id,
            intensity: default_secondary_intensity(),
        }
    }
}

/// A single slot inscription using the new RootWord-based model.
///
/// Secondary Ancient Words applied to one selected ability slot. The Root Word
/// is stored once on [`WeaponInscription`] and is shared by all three slots.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SlotInscription {
    /// Secondary words that modify or enhance the item's Root Word.
    pub secondary_words: Vec<SecondaryWord>,
}

impl SlotInscription {
    pub fn is_empty(&self) -> bool {
        self.secondary_words.is_empty()
    }

    /// Add a secondary word to this inscription.
    pub fn with_secondary(mut self, word: SecondaryWord) -> Self {
        self.secondary_words.push(word);
        self
    }
}

/// Complete weapon inscription using the new RootWord-based model.
///
/// Replaces [`legacy::WeaponInscriptions`] once migration is complete.
/// Contains one [`SlotInscription`] per ability slot.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WeaponInscription {
    /// One Root Word shared by Primary, Secondary and Ultimate.
    pub root_word: Option<RootWordId>,
    pub primary: SlotInscription,
    pub secondary: SlotInscription,
    pub ultimate: SlotInscription,
}

impl WeaponInscription {
    pub fn get(&self, slot: AbilitySlot) -> &SlotInscription {
        match slot {
            AbilitySlot::Primary => &self.primary,
            AbilitySlot::Secondary => &self.secondary,
            AbilitySlot::Ultimate => &self.ultimate,
        }
    }

    pub fn get_mut(&mut self, slot: AbilitySlot) -> &mut SlotInscription {
        match slot {
            AbilitySlot::Primary => &mut self.primary,
            AbilitySlot::Secondary => &mut self.secondary,
            AbilitySlot::Ultimate => &mut self.ultimate,
        }
    }

    /// Check if all slots are empty (no inscriptions at all).
    pub fn is_fresh(&self) -> bool {
        self.root_word.is_none()
            && self.primary.is_empty()
            && self.secondary.is_empty()
            && self.ultimate.is_empty()
    }
}

/// Ability-level inscription for fine-grained ability customization.
///
/// Unlike [`WeaponInscription`] which covers all three slots, this represents
/// inscription data for a single resolved ability. Useful for blueprint
/// construction and spell resolution.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AbilityInscription {
    /// Secondary words that modify the item's shared Root Word for this ability.
    pub secondary_words: Vec<SecondaryWord>,
}

impl AbilityInscription {
    pub fn is_empty(&self) -> bool {
        self.secondary_words.is_empty()
    }
}

/// Enum dispatching to the appropriate inscription type for an item.
///
/// Allows [`ItemInstance`] to hold either new-style or legacy inscriptions
/// during the migration period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemInscription {
    /// New RootWord-based weapon inscription.
    Weapon(WeaponInscription),
    /// New independent armor inscription. Armor does not use fake Q/W/E slots.
    Armor(ArmorInscription),
}

impl ItemInscription {
    pub fn is_empty(&self) -> bool {
        match self {
            ItemInscription::Weapon(w) => w.is_fresh(),
            ItemInscription::Armor(a) => a.is_empty(),
        }
    }

    /// Create a new empty weapon inscription (convenience constructor).
    pub fn new_weapon() -> Self {
        ItemInscription::Weapon(WeaponInscription::default())
    }
}

/// Independent inscription carried by Helmet, Chest and Shoes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ArmorInscription {
    pub root_word: Option<RootWordId>,
    pub secondary_words: Vec<SecondaryWord>,
}

impl ArmorInscription {
    pub fn is_empty(&self) -> bool {
        self.root_word.is_none() && self.secondary_words.is_empty()
    }
}

impl Default for ItemInscription {
    fn default() -> Self {
        ItemInscription::new_weapon()
    }
}

/// Quanta "frase" può reggere un'arma — dato statico del catalogo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuneProfile {
    pub capacity: u32,
    /// 0.0-1.0. Reserved for the stability trade-off defined by the plan.
    pub stability: f32,
}
