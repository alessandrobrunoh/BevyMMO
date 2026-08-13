//! `Inscription`/`WeaponInscriptions` — l'incisione runica, attaccata
//! all'ESEMPLARE fisico dell'arma (`ItemInstance`, non al player: due Flame
//! Staff possono essere incisi in modo diverso). `RuneProfile` è invece dato
//! statico del catalogo (quanta Capacità/Stabilità/Affinità ha QUEL TIPO
//! di arma), letto da `Item::rune_profile`.

use serde::{Deserialize, Serialize};

use super::ancient_word::{AncientWordId, AncientWordRegistry};
use super::base_ability::{AbilityId, BaseAbilityRegistry};
use super::essence::{EssenceId, EssenceRegistry};
use super::modifier::{ModifierId, ModifierRegistry};
use super::slot::AbilitySlot;
use super::weapon_abilities::WeaponAbilities;

/// L'incisione di UNO slot (Primary/Secondary/Ultimate).
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
fn inscription_cost(
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

/// Valida UNO slot: ogni Glifo deve esistere nei registry e rispettare i tag
/// richiesti dalla `BaseAbility` di quello slot. Non controlla ancora la
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

/// Valida l'intera incisione di un'arma: compatibilità tag per ogni slot +
/// Capacità Runica totale (condivisa fra i tre slot).
pub fn validate_weapon_inscriptions(
    inscriptions: &WeaponInscriptions,
    abilities: &WeaponAbilities,
    profile: &RuneProfile,
    ability_registry: &BaseAbilityRegistry,
    essences: &EssenceRegistry,
    modifiers: &ModifierRegistry,
    ancient_words: &AncientWordRegistry,
) -> Result<(), InscriptionError> {
    let mut total_cost = 0;
    for slot in AbilitySlot::ALL {
        let inscription = inscriptions.get(slot);
        let ability_id = abilities.get(slot);
        validate_slot(inscription, ability_id, ability_registry, essences, modifiers, ancient_words)?;
        total_cost += inscription_cost(inscription, profile, essences, modifiers, ancient_words);
    }
    if total_cost > profile.capacity {
        return Err(InscriptionError::CapacityExceeded { cost: total_cost, capacity: profile.capacity });
    }
    Ok(())
}
