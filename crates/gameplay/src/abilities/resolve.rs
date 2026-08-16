//! L'interprete: `BaseAbility` (gesto) + `Inscription` (Glifi) →
//! effetto finale, tramite lo stesso `SpellCastContext` già usato dalle
//! spell "classiche" — riusa tutta la pipeline di cast/rete esistente.

use super::ancient_word::AncientWordRegistry;
use super::base_ability::{AbilityParams, ArcBaseAbility, BaseAbilityRegistry};
use super::essence::EssenceRegistry;
use super::inscription::WeaponInscriptions;
use super::known_glyphs::KnownGlyphs;
use super::modifier::ModifierRegistry;
use super::slot::AbilitySlot;
use super::weapon_abilities::{resolve_active_ability, AbilitySelection, WeaponAbilities};
use crate::spells::context::SpellCastContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastBlockedReason {
    /// Lo slot è incisione con almeno un Glifo che il caster non conosce:
    /// blocco totale, come deciso — nessun cast, non solo "senza effetto".
    UnknownGlyph,
    /// L'`AbilityId`/`EssenceId`/... incisi non esistono più nei registry,
    /// o `WeaponAbilities` non offre nessun gesto per lo slot (dati
    /// incoerenti — non dovrebbe succedere con contenuti registrati
    /// correttamente, ma viene gestito senza panic).
    MissingRegistryEntry,
}

/// Applica in sequenza i Modificatori conosciuti ai parametri base
/// (§14-24). I Modificatori NON conosciuti sono ignorati silenziosamente:
/// solo l'Essenza sconosciuta/il blocco totale sono un "hard fail" — qui
/// stiamo già dentro `cast_inscribed_slot`, che ha già verificato
/// `fully_knows` prima di chiamare questa funzione, quindi in pratica ogni
/// modificatore qui è sempre conosciuto. La funzione resta comunque
/// difensiva (ignora ciò che non trova nel registry).
pub fn resolve_ability_params(
    base: AbilityParams,
    inscription_modifiers: &[super::modifier::ModifierId],
    modifiers: &ModifierRegistry,
) -> AbilityParams {
    let mut params = base;
    for modifier_id in inscription_modifiers {
        if let Some(modifier) = modifiers.get(modifier_id) {
            modifier.transform(&mut params);
        }
    }
    params
}

/// Il gesto attivo su uno slot e i suoi parametri già modificati — tutto ciò
/// che serve per sapere COSA e DOVE colpirà, senza lanciarlo.
pub struct SlotPreview {
    pub ability: ArcBaseAbility,
    pub params: AbilityParams,
}

/// Prima metà di [`cast_inscribed_slot`]: risolve il gesto attivo, verifica i
/// Glifi conosciuti e applica i Modificatori, ma **non emette nulla**.
///
/// Esiste perché il client possa disegnare l'anteprima di mira con
/// esattamente la stessa risoluzione che il server userà un istante dopo per
/// applicare l'effetto: `cast_inscribed_slot` ci passa sopra, quindi le due
/// strade non possono divergere. Un `Err` significa slot bloccato — il
/// preview lo colora di conseguenza invece di disegnare un'area che non
/// arriverà mai.
pub fn resolve_slot_preview(
    slot: AbilitySlot,
    abilities: &WeaponAbilities,
    selection: &AbilitySelection,
    inscriptions: &WeaponInscriptions,
    known: &KnownGlyphs,
    ability_registry: &BaseAbilityRegistry,
    modifier_registry: &ModifierRegistry,
) -> Result<SlotPreview, CastBlockedReason> {
    let inscription = inscriptions.get(slot);
    if !known.fully_knows(inscription) {
        return Err(CastBlockedReason::UnknownGlyph);
    }

    let Some(ability_id) = resolve_active_ability(slot, abilities, selection) else {
        return Err(CastBlockedReason::MissingRegistryEntry);
    };
    let ability = ability_registry
        .get(ability_id)
        .ok_or(CastBlockedReason::MissingRegistryEntry)?;

    let params = resolve_ability_params(
        ability.base_params(),
        &inscription.modifiers,
        modifier_registry,
    );

    Ok(SlotPreview { ability, params })
}

/// Lancia lo slot `slot` dell'arma incisa `inscriptions`.
///
/// Risolve prima il gesto EFFETTIVAMENTE attivo per lo slot (per
/// Primary/Secondary, la scelta del giocatore fra le opzioni offerte —
/// vedi [`resolve_active_ability`]; per Ultimate, l'unico gesto disponibile).
///
/// Blocco totale (§ scelta di design confermata): se anche un solo Glifo
/// incastonato in quello slot non è nel `KnownGlyphs` del caster, la
/// funzione ritorna `Err` e non emette nessun effetto — non un fallback
/// "solo fisico". Un'arma trovata già incisa da uno sconosciuto resta
/// inutilizzabile su quello slot finché non impari TUTTI i Glifi coinvolti.
#[allow(clippy::too_many_arguments)]
pub fn cast_inscribed_slot(
    slot: AbilitySlot,
    abilities: &WeaponAbilities,
    selection: &AbilitySelection,
    inscriptions: &WeaponInscriptions,
    known: &KnownGlyphs,
    ability_registry: &BaseAbilityRegistry,
    essence_registry: &EssenceRegistry,
    modifier_registry: &ModifierRegistry,
    ancient_word_registry: &AncientWordRegistry,
    ctx: &mut SpellCastContext,
) -> Result<(), CastBlockedReason> {
    let SlotPreview { ability, params } = resolve_slot_preview(
        slot,
        abilities,
        selection,
        inscriptions,
        known,
        ability_registry,
        modifier_registry,
    )?;
    let inscription = inscriptions.get(slot);

    match &inscription.essence {
        Some(essence_id) => {
            let essence = essence_registry.get(essence_id).ok_or(CastBlockedReason::MissingRegistryEntry)?;
            essence.manifest(ability.as_ref(), &params, ctx);
        }
        None => ability.default_manifestation(&params, ctx),
    }

    if let Some(word_id) = &inscription.ancient_word {
        let word = ancient_word_registry.get(word_id).ok_or(CastBlockedReason::MissingRegistryEntry)?;
        word.post_process(ability.as_ref(), &params, ctx);
    }

    Ok(())
}

