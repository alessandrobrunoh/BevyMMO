//! Character knowledge models — permanent vocabularies that survive full-loot.
//!
//! Two coexisting knowledge systems:
//! - [`KnownGlyphs`] — legacy Essence/Modifier/AncientWord vocabulary (§46-47)
//! - [`KnownAncientLanguage`] — new RootWord-based vocabulary for the
//!   RootWord inscription model (`SlotInscription`/`ItemInscription`)

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::ancient_word::AncientWordId;
use super::base_ability::AbilityId;
use super::essence::EssenceId;
use super::inscription::{ItemInscription, SlotInscription};
use super::modifier::ModifierId;
use super::root_word::RootWordId;

// Re-export legacy Inscription for KnownGlyphs compatibility
use super::inscription::Inscription;

// ══════════════════════════════════════════════════════════════════
// LEGACY KNOWLEDGE MODEL (Essence/Modifier/AncientWord)
// ══════════════════════════════════════════════════════════════════

/// Legacy character vocabulary: which Essences/Modifiers/AncientWords the
/// character can read. Permanent, survives full-loot (§46-47).
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KnownGlyphs {
    pub essences: HashSet<EssenceId>,
    pub modifiers: HashSet<ModifierId>,
    pub ancient_words: HashSet<AncientWordId>,
}

impl KnownGlyphs {
    /// True if every glyph in `inscription` is known.
    /// Used for "total slot block": if even one glyph is missing,
    /// the entire slot inscription is unusable.
    pub fn fully_knows(&self, inscription: &Inscription) -> bool {
        inscription
            .essence
            .as_ref()
            .is_none_or(|id| self.essences.contains(id))
            && inscription
                .modifiers
                .iter()
                .all(|id| self.modifiers.contains(id))
            && inscription
                .ancient_word
                .as_ref()
                .is_none_or(|id| self.ancient_words.contains(id))
    }
}

// ══════════════════════════════════════════════════════════════════
// NEW KNOWLEDGE MODEL (RootWord-based)
// ══════════════════════════════════════════════════════════════════

/// Character knowledge for the RootWord-based inscription system.
///
/// Tracks which Root Words, Ancient Words, and base abilities the
/// character understands. This is the knowledge check gate for
/// [`SlotInscription`] and [`ItemInscription::Weapon`].
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KnownAncientLanguage {
    /// Root Words the character can use as primary inscription identity.
    pub root_words: HashSet<RootWordId>,
    /// Ancient Words the character can apply as modifiers (shared with legacy system).
    pub ancient_words: HashSet<AncientWordId>,
    /// Base abilities whose semantics the character understands.
    /// Required to validate that a Root Word's effect is meaningful
    /// for the ability it's inscribed upon.
    pub base_abilities: HashSet<AbilityId>,
}

impl KnownAncientLanguage {
    /// Check if a [`SlotInscription`] is fully known.
    ///
    /// Returns `true` only if:
    /// - Every secondary Ancient Word is in `ancient_words`
    pub fn knows_slot(&self, slot: &SlotInscription) -> bool {
        slot.secondary_words
            .iter()
            .all(|sw| self.ancient_words.contains(&sw.word_id))
    }

    /// Check if an [`ItemInscription`] is fully known.
    ///
    /// For [`ItemInscription::Weapon`]: checks all three slots via [`Self::knows_slot`].
    /// For [`ItemInscription::Legacy`]: always returns `false` — legacy inscriptions
    /// must use [`KnownGlyphs::fully_knows`] instead.
    pub fn knows_item_inscription(&self, item: &ItemInscription) -> bool {
        match item {
            ItemInscription::Weapon(weapon) => {
                weapon
                    .root_word
                    .as_ref()
                    .is_none_or(|id| self.root_words.contains(id))
                    && self.knows_slot(&weapon.primary)
                    && self.knows_slot(&weapon.secondary)
                    && self.knows_slot(&weapon.ultimate)
            }
            ItemInscription::Armor(armor) => {
                armor
                    .root_word
                    .as_ref()
                    .is_none_or(|id| self.root_words.contains(id))
                    && armor
                        .secondary_words
                        .iter()
                        .all(|word| self.ancient_words.contains(&word.word_id))
            }
            ItemInscription::Legacy(_) => false,
        }
    }

    /// Check if a specific Root Word is known.
    pub fn knows_root_word(&self, id: &RootWordId) -> bool {
        self.root_words.contains(id)
    }

    /// Check if a specific Ancient Word is known.
    pub fn knows_ancient_word(&self, id: &AncientWordId) -> bool {
        self.ancient_words.contains(id)
    }

    /// Check if a base ability's semantics are understood.
    pub fn knows_base_ability(&self, id: &AbilityId) -> bool {
        self.base_abilities.contains(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── KnownGlyphs (legacy) tests ────────────────────────────────

    #[test]
    fn fully_knows_true_for_empty_inscription() {
        let known = KnownGlyphs::default();
        assert!(known.fully_knows(&Inscription::default()));
    }

    #[test]
    fn fully_knows_false_when_essence_missing() {
        let known = KnownGlyphs::default();
        let inscription = Inscription {
            essence: Some(EssenceId::new("fuoco")),
            ..Default::default()
        };
        assert!(!known.fully_knows(&inscription));
    }

    #[test]
    fn fully_knows_true_when_every_glyph_is_known() {
        let mut known = KnownGlyphs::default();
        known.essences.insert(EssenceId::new("fuoco"));
        known.modifiers.insert(ModifierId::new("espandere"));
        let inscription = Inscription {
            essence: Some(EssenceId::new("fuoco")),
            modifiers: vec![ModifierId::new("espandere")],
            ancient_word: None,
        };
        assert!(known.fully_knows(&inscription));
    }

    #[test]
    fn fully_knows_false_when_one_modifier_among_several_is_unknown() {
        let mut known = KnownGlyphs::default();
        known.essences.insert(EssenceId::new("fuoco"));
        known.modifiers.insert(ModifierId::new("espandere"));
        // "amplificare" non è nel vocabolario.
        let inscription = Inscription {
            essence: Some(EssenceId::new("fuoco")),
            modifiers: vec![ModifierId::new("espandere"), ModifierId::new("amplificare")],
            ancient_word: None,
        };
        assert!(!known.fully_knows(&inscription));
    }

    // ── KnownAncientLanguage (new) tests ─────────────────────────

    use crate::abilities::SecondaryWord;
    use crate::abilities::WeaponInscription;

    #[test]
    fn knows_slot_true_for_empty_slot() {
        let known = KnownAncientLanguage::default();
        assert!(known.knows_slot(&SlotInscription::default()));
    }

    #[test]
    fn knows_slot_false_when_secondary_word_unknown() {
        let known = KnownAncientLanguage::default();
        let slot = SlotInscription::default()
            .with_secondary(SecondaryWord::new(AncientWordId::new("empower")));
        assert!(!known.knows_slot(&slot));
    }

    #[test]
    fn knows_slot_true_with_known_secondary() {
        let mut known = KnownAncientLanguage::default();
        known.ancient_words.insert(AncientWordId::new("empower"));
        let slot = SlotInscription::default()
            .with_secondary(SecondaryWord::new(AncientWordId::new("empower")));
        assert!(known.knows_slot(&slot));
    }

    #[test]
    fn knows_item_inscription_true_for_empty_weapon() {
        let known = KnownAncientLanguage::default();
        let item = ItemInscription::new_weapon();
        assert!(known.knows_item_inscription(&item));
    }

    #[test]
    fn knows_item_inscription_false_for_legacy() {
        let known = KnownAncientLanguage::default();
        let item = ItemInscription::Legacy(
            crate::abilities::inscription::legacy::WeaponInscriptions::default(),
        );
        assert!(!known.knows_item_inscription(&item));
    }

    #[test]
    fn knows_item_inscription_checks_all_slots() {
        let mut known = KnownAncientLanguage::default();
        known.root_words.insert(RootWordId::from("damage"));

        let weapon = WeaponInscription {
            root_word: Some(RootWordId::from("damage")),
            ..Default::default()
        };

        let item = ItemInscription::Weapon(weapon);
        assert!(known.knows_item_inscription(&item));
    }

    #[test]
    fn knows_item_inscription_false_when_any_slot_fails() {
        let known = KnownAncientLanguage::default();
        // "missing" Root Word is not learned.
        let weapon = WeaponInscription {
            root_word: Some(RootWordId::from("missing")),
            ..Default::default()
        };

        let item = ItemInscription::Weapon(weapon);
        assert!(!known.knows_item_inscription(&item));
    }

    #[test]
    fn individual_knowledge_checks() {
        let mut known = KnownAncientLanguage::default();
        known.root_words.insert(RootWordId::from("damage"));
        known.ancient_words.insert(AncientWordId::new("eco"));
        known.base_abilities.insert(AbilityId::new("slash"));

        assert!(known.knows_root_word(&RootWordId::from("damage")));
        assert!(!known.knows_root_word(&RootWordId::from("heal")));
        assert!(known.knows_ancient_word(&AncientWordId::new("eco")));
        assert!(!known.knows_ancient_word(&AncientWordId::new("dividere")));
        assert!(known.knows_base_ability(&AbilityId::new("slash")));
        assert!(!known.knows_base_ability(&AbilityId::new("thrust")));
    }
}
