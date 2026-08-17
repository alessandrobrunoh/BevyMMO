//! Character knowledge models for the Root Word and Ancient Word vocabulary.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::ancient_word::AncientWordId;
use super::base_ability::AbilityId;
use super::inscription::{ItemInscription, SlotInscription};
use super::root_word::RootWordId;

/// Character knowledge for the RootWord-based inscription system.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KnownAncientLanguage {
    /// Root Words the character can use as primary inscription identity.
    pub root_words: HashSet<RootWordId>,
    /// Ancient Words the character can apply as secondary words.
    pub ancient_words: HashSet<AncientWordId>,
    /// Base abilities whose semantics the character understands.
    pub base_abilities: HashSet<AbilityId>,
}

impl KnownAncientLanguage {
    pub fn knows_slot(&self, slot: &SlotInscription) -> bool {
        slot.secondary_words
            .iter()
            .all(|word| self.ancient_words.contains(&word.word_id))
    }

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
        }
    }

    pub fn knows_root_word(&self, id: &RootWordId) -> bool {
        self.root_words.contains(id)
    }

    pub fn knows_ancient_word(&self, id: &AncientWordId) -> bool {
        self.ancient_words.contains(id)
    }

    pub fn knows_base_ability(&self, id: &AbilityId) -> bool {
        self.base_abilities.contains(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{SecondaryWord, WeaponInscription};

    #[test]
    fn knows_slot_true_for_empty_slot() {
        assert!(KnownAncientLanguage::default().knows_slot(&SlotInscription::default()));
    }

    #[test]
    fn knows_slot_false_when_secondary_word_unknown() {
        let slot = SlotInscription::default()
            .with_secondary(SecondaryWord::new(AncientWordId::new("echo")));
        assert!(!KnownAncientLanguage::default().knows_slot(&slot));
    }

    #[test]
    fn knows_item_inscription_checks_root_word_and_all_slots() {
        let mut known = KnownAncientLanguage::default();
        known.root_words.insert(RootWordId::from("damage"));
        let item = ItemInscription::Weapon(WeaponInscription {
            root_word: Some(RootWordId::from("damage")),
            ..Default::default()
        });
        assert!(known.knows_item_inscription(&item));
    }
}
