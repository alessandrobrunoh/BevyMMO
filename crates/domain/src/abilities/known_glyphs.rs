//! `KnownGlyphs` — il Vocabolario del personaggio: quali Essenze/Modificatori/
//! Parole Antiche sa leggere. Permanente, sopravvive al full-loot
//! dell'equipaggiamento (§46-47 del design) — per questo è un component
//! separato dall'arma/incisione, non un dato dell'item.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::ancient_word::AncientWordId;
use super::essence::EssenceId;
use super::inscription::Inscription;
use super::modifier::ModifierId;

#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KnownGlyphs {
    pub essences: HashSet<EssenceId>,
    pub modifiers: HashSet<ModifierId>,
    pub ancient_words: HashSet<AncientWordId>,
}

impl KnownGlyphs {
    /// Vero se ogni singolo Glifo usato in `inscription` è conosciuto.
    /// Usato per il "blocco totale dello slot": se anche un Glifo manca,
    /// l'intera incisione di quello slot è inutilizzabile (vedi
    /// `crate::abilities::resolve::cast_inscribed_slot`).
    pub fn fully_knows(&self, inscription: &Inscription) -> bool {
        inscription
            .essence
            .as_ref()
            .is_none_or(|id| self.essences.contains(id))
            && inscription.modifiers.iter().all(|id| self.modifiers.contains(id))
            && inscription
                .ancient_word
                .as_ref()
                .is_none_or(|id| self.ancient_words.contains(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fully_knows_true_for_empty_inscription() {
        let known = KnownGlyphs::default();
        assert!(known.fully_knows(&Inscription::default()));
    }

    #[test]
    fn fully_knows_false_when_essence_missing() {
        let known = KnownGlyphs::default();
        let inscription = Inscription { essence: Some(EssenceId::new("fuoco")), ..Default::default() };
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
}
