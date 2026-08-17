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
//! ## Legacy Model (pre-RootWord)
//!
//! The original [`Inscription`]/[`WeaponInscriptions`] types are preserved
//! under the `legacy` module for backward compatibility. These use the old
//! Essence/Modifier/AncientWord structure and should be migrated incrementally.
//!
//! ## Architecture
//!
//! ```text
//! ItemInstance
//! └── inscriptions: Option<ItemInscription>  (NEW: enum dispatch)
//!     ├── Weapon(WeaponInscription)         (NEW: RootWord + secondaries)
//!     └── Legacy(WeaponInscriptions)        (OLD: Essence/Modifier/AncientWord)
//! ```

use serde::{Deserialize, Serialize};

use super::ancient_word::{AncientWordId, AncientWordRegistry};
use super::base_ability::{AbilityId, BaseAbilityRegistry};
use super::essence::{EssenceId, EssenceRegistry};
use super::modifier::{ModifierId, ModifierRegistry};
use super::root_word::RootWordId;
use super::slot::AbilitySlot;
use super::weapon_abilities::{resolve_active_ability, AbilitySelection, WeaponAbilities};

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
    /// Legacy Essence/Modifier/AncientWord inscription (pre-RootWord).
    Legacy(legacy::WeaponInscriptions), // Note: legacy::WeaponInscriptions is the old type
}

impl ItemInscription {
    pub fn is_empty(&self) -> bool {
        match self {
            ItemInscription::Weapon(w) => w.is_fresh(),
            ItemInscription::Armor(a) => a.is_empty(),
            ItemInscription::Legacy(l) => {
                l.primary.is_empty() && l.secondary.is_empty() && l.ultimate.is_empty()
            }
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

// ══════════════════════════════════════════════════════════════════
// LEGACY INSCRIPTION MODEL (pre-RootWord)
// ══════════════════════════════════════════════════════════════════

/// Legacy inscription types preserved for backward compatibility.
///
/// These use the original Essence/Modifier/AncientWord model and should
/// be incrementally migrated to the new RootWord-based types above.
pub mod legacy {
    use serde::{Deserialize, Serialize};

    use crate::abilities::ancient_word::AncientWordId;
    use crate::abilities::essence::EssenceId;
    use crate::abilities::modifier::ModifierId;
    use crate::abilities::slot::AbilitySlot;

    /// L'incisione di UNO slot (Primary/Secondary/Ultimate).
    ///
    /// **Legacy type** — prefer [`super::SlotInscription`] for new code.
    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Inscription {
        pub essence: Option<EssenceId>,
        pub modifiers: Vec<ModifierId>,
        pub ancient_word: Option<AncientWordId>,
    }

    impl Inscription {
        pub fn is_empty(&self) -> bool {
            self.essence.is_none() && self.modifiers.is_empty() && self.ancient_word.is_none()
        }
    }

    /// Le tre incisioni di un esemplare d'arma.
    ///
    /// **Legacy type** — prefer [`super::WeaponInscription`] for new code.
    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct WeaponInscriptions {
        pub primary: Inscription,
        pub secondary: Inscription,
        pub ultimate: Inscription,
    }

    impl WeaponInscriptions {
        pub fn get(&self, slot: AbilitySlot) -> &Inscription {
            match slot {
                AbilitySlot::Primary => &self.primary,
                AbilitySlot::Secondary => &self.secondary,
                AbilitySlot::Ultimate => &self.ultimate,
            }
        }
        pub fn get_mut(&mut self, slot: AbilitySlot) -> &mut Inscription {
            match slot {
                AbilitySlot::Primary => &mut self.primary,
                AbilitySlot::Secondary => &mut self.secondary,
                AbilitySlot::Ultimate => &mut self.ultimate,
            }
        }
    }
}

// Re-export legacy types at module level for backward compatibility
pub use legacy::{Inscription, WeaponInscriptions};

/// Type alias for clarity when referencing the legacy type.
pub type LegacyWeaponInscriptions = WeaponInscriptions;

/// Quanta "frase" può reggere un'arma — dato statico del catalogo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuneProfile {
    pub capacity: u32,
    /// 0.0-1.0. Riservato: oggi non influenza ancora nulla nel motore
    /// (§43 del design lascia aperta la scelta su cosa penalizzare).
    pub stability: f32,
    pub affinity: Option<EssenceId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InscriptionError {
    UnknownAbility(AbilityId),
    UnknownEssence(EssenceId),
    UnknownModifier(ModifierId),
    UnknownAncientWord(AncientWordId),
    IncompatibleModifier(ModifierId),
    IncompatibleAncientWord(AncientWordId),
    CapacityExceeded { cost: u32, capacity: u32 },
}

/// Costo in Capacità Runica di un'incisione, scontato dell'eventuale
/// Affinità dell'arma (§44: "su quest'arma costa 1 invece di 2").
///
/// Pubblica perché la scheda item la mostra al giocatore: se la UI
/// ricalcolasse il costo per conto suo, lo sconto di Affinità potrebbe
/// divergere da quello che la validazione applica davvero.
pub fn inscription_cost(
    inscription: &Inscription,
    profile: &RuneProfile,
    essences: &EssenceRegistry,
    modifiers: &ModifierRegistry,
    ancient_words: &AncientWordRegistry,
) -> u32 {
    let essence_cost = inscription
        .essence
        .as_ref()
        .and_then(|id| essences.get(id))
        .map(|essence| {
            let base = essence.rune_cost();
            if profile.affinity.as_ref() == inscription.essence.as_ref() {
                base.saturating_sub(1)
            } else {
                base
            }
        })
        .unwrap_or(0);

    let modifiers_cost: u32 = inscription
        .modifiers
        .iter()
        .filter_map(|id| modifiers.get(id))
        .map(|modifier| modifier.rune_cost())
        .sum();

    let word_cost = inscription
        .ancient_word
        .as_ref()
        .and_then(|id| ancient_words.get(id))
        .map(|word| word.rune_cost())
        .unwrap_or(0);

    essence_cost + modifiers_cost + word_cost
}

/// Valida UNO slot contro il gesto EFFETTIVAMENTE attivo (dopo aver
/// risolto `AbilitySelection` — un gesto scelto dal giocatore può avere tag
/// diversi dall'altra opzione dello stesso slot). Non controlla ancora la
/// Capacità totale — quella si valida sull'intera `WeaponInscriptions`
/// (§39-41: la Capacità è condivisa fra Primary/Secondary/Ultimate).
fn validate_slot(
    inscription: &Inscription,
    ability_id: &AbilityId,
    ability_registry: &BaseAbilityRegistry,
    essences: &EssenceRegistry,
    modifiers: &ModifierRegistry,
    ancient_words: &AncientWordRegistry,
) -> Result<(), InscriptionError> {
    let ability = ability_registry
        .get(ability_id)
        .ok_or_else(|| InscriptionError::UnknownAbility(ability_id.clone()))?;

    if let Some(essence_id) = &inscription.essence {
        if !essences.contains(essence_id) {
            return Err(InscriptionError::UnknownEssence(essence_id.clone()));
        }
    }

    for modifier_id in &inscription.modifiers {
        let modifier = modifiers
            .get(modifier_id)
            .ok_or_else(|| InscriptionError::UnknownModifier(modifier_id.clone()))?;
        if !ability.has_tag(modifier.required_tag()) {
            return Err(InscriptionError::IncompatibleModifier(modifier_id.clone()));
        }
    }

    if let Some(word_id) = &inscription.ancient_word {
        let word = ancient_words
            .get(word_id)
            .ok_or_else(|| InscriptionError::UnknownAncientWord(word_id.clone()))?;
        if !ability.has_tag(word.required_tag()) {
            return Err(InscriptionError::IncompatibleAncientWord(word_id.clone()));
        }
    }

    Ok(())
}

/// Valida l'intera incisione di un'arma: compatibilità tag per ogni slot
/// (contro il gesto EFFETTIVAMENTE selezionato, non un gesto fisso — vedi
/// [`resolve_active_ability`]) + Capacità Runica totale (condivisa fra i tre
/// slot).
#[allow(clippy::too_many_arguments)]
pub fn validate_weapon_inscriptions(
    inscriptions: &WeaponInscriptions,
    abilities: &WeaponAbilities,
    selection: &AbilitySelection,
    profile: &RuneProfile,
    ability_registry: &BaseAbilityRegistry,
    essences: &EssenceRegistry,
    modifiers: &ModifierRegistry,
    ancient_words: &AncientWordRegistry,
) -> Result<(), InscriptionError> {
    let mut total_cost = 0;
    for slot in AbilitySlot::ALL {
        let inscription = inscriptions.get(slot);
        // `WeaponAbilities::new` guarantees Primary/Secondary are non-empty
        // and Ultimate always has its one gesture, so this only ever comes
        // back `None` for a malformed `WeaponAbilities` — skip rather than
        // panic on data that shouldn't exist.
        let Some(ability_id) = resolve_active_ability(slot, abilities, selection) else {
            continue;
        };
        validate_slot(inscription, ability_id, ability_registry, essences, modifiers, ancient_words)?;
        total_cost += inscription_cost(inscription, profile, essences, modifiers, ancient_words);
    }
    if total_cost > profile.capacity {
        return Err(InscriptionError::CapacityExceeded { cost: total_cost, capacity: profile.capacity });
    }
    Ok(())
}
