//! L'interprete: `BaseAbility` (gesto) + `Inscription` (Glifi) →
//! effetto finale, tramite lo stesso `SpellCastContext` già usato dalle
//! spell "classiche" — riusa tutta la pipeline di cast/rete esistente.

use super::ancient_word::AncientWordRegistry;
use super::base_ability::{AbilityParams, AbilityTag, ArcBaseAbility, BaseAbilityRegistry};
use super::blueprint::AbilityBlueprint;
use super::essence::EssenceRegistry;
use super::inscription::{ArmorInscription, WeaponInscription, WeaponInscriptions};
use super::known_glyphs::{KnownAncientLanguage, KnownGlyphs};
use super::modifier::ModifierRegistry;
use super::root_word::RootWordRegistry;
use super::slot::AbilitySlot;
use super::weapon_abilities::{resolve_active_ability, AbilitySelection, WeaponAbilities};
use crate::items::Item;
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
    UnknownRootWord,
    UnknownAncientWord,
    IncompatibleAncientWord,
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
    pub blueprint: AbilityBlueprint,
    /// Compatibility view of `blueprint.params` for existing callers.
    pub params: AbilityParams,
}

fn manifest_blueprint(preview: &SlotPreview, ctx: &mut SpellCastContext) {
    preview.ability.default_manifestation(&preview.params, ctx);
    if matches!(
        preview.blueprint.execution,
        super::blueprint::BlueprintExecution::Echo
    ) && preview.blueprint.has_tag(AbilityTag::EchoCompatible)
    {
        // Echo is a second manifestation of the already-resolved blueprint;
        // it never re-enters item/root/word resolution, so recursive echoes
        // are impossible.
        preview.ability.default_manifestation(&preview.params, ctx);
    }
}

/// Shared tail of [`cast_armor_inscribed_ability`] and
/// [`cast_root_inscribed_slot`]: both are `resolve_* → manifest_blueprint →
/// Ok(())`, differing only in which `resolve_*` produced the preview. Takes
/// the already-evaluated `Result` rather than a closure — the caller always
/// wants to resolve eagerly before manifesting, so there is no laziness to
/// preserve, and this avoids a boxed/generic closure parameter for no
/// behavioral gain.
fn manifest_from_preview(
    preview: Result<SlotPreview, CastBlockedReason>,
    ctx: &mut SpellCastContext,
) -> Result<(), CastBlockedReason> {
    manifest_blueprint(&preview?, ctx);
    Ok(())
}

/// Applies each already-sorted secondary Ancient Word to `blueprint`: checks
/// the caster knows it, checks it is tag-compatible with the blueprint so
/// far, then lets it transform the blueprint — erroring identically on an
/// unknown or incompatible word either way.
///
/// Shared by the weapon (Root Word inscription, secondary words sorted by
/// phase then word id) and armor (sorted by word id only) paths; the sort
/// order is the only real difference between them, so it stays the
/// caller's job and this only takes the already-sorted slice.
fn apply_secondary_words(
    blueprint: &mut AbilityBlueprint,
    words: &[super::inscription::SecondaryWord],
    known: &KnownAncientLanguage,
    ancient_words: &AncientWordRegistry,
) -> Result<(), CastBlockedReason> {
    for secondary in words {
        if !known.knows_ancient_word(&secondary.word_id) {
            return Err(CastBlockedReason::UnknownAncientWord);
        }
        let word = ancient_words
            .get(&secondary.word_id)
            .ok_or(CastBlockedReason::UnknownAncientWord)?;
        if !word.metadata().is_compatible_with(&blueprint.tags) {
            return Err(CastBlockedReason::IncompatibleAncientWord);
        }
        word.transform_blueprint(blueprint);
    }
    Ok(())
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
    item: Option<&dyn Item>,
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

    // Item-aware resolution: use the item's blueprint transform when available
    // (e.g., ConduitStaffT4 changes execution to Charge).
    let mut blueprint = match item {
        Some(i) => i.ability_blueprint(ability.as_ref()),
        None => ability.blueprint(),
    };
    blueprint.params =
        resolve_ability_params(blueprint.params, &inscription.modifiers, modifier_registry);
    let params = blueprint.params;

    Ok(SlotPreview {
        ability,
        blueprint,
        params,
    })
}

/// Resolves the additive RootWord inscription path. The legacy resolver above
/// remains available while persisted characters migrate.
#[allow(clippy::too_many_arguments)]
pub fn resolve_root_inscribed_slot(
    slot: AbilitySlot,
    abilities: &WeaponAbilities,
    selection: &AbilitySelection,
    inscription: &WeaponInscription,
    known: &KnownAncientLanguage,
    ability_registry: &BaseAbilityRegistry,
    root_words: &RootWordRegistry,
    ancient_words: &AncientWordRegistry,
    item: Option<&dyn Item>,
) -> Result<SlotPreview, CastBlockedReason> {
    let Some(ability_id) = resolve_active_ability(slot, abilities, selection) else {
        return Err(CastBlockedReason::MissingRegistryEntry);
    };
    let ability = ability_registry
        .get(ability_id)
        .ok_or(CastBlockedReason::MissingRegistryEntry)?;

    if !known.knows_root_word(
        inscription
            .root_word
            .as_ref()
            .ok_or(CastBlockedReason::UnknownRootWord)?,
    ) {
        return Err(CastBlockedReason::UnknownRootWord);
    }

    let mut blueprint = match item {
        Some(item) => item.ability_blueprint(ability.as_ref()),
        None => ability.blueprint(),
    };

    let root_id = inscription
        .root_word
        .as_ref()
        .ok_or(CastBlockedReason::UnknownRootWord)?;
    let root = root_words
        .get(root_id)
        .ok_or(CastBlockedReason::UnknownRootWord)?;
    let root_params = blueprint.params;
    root.apply_to_blueprint(&mut blueprint, &root_params);

    let mut words = inscription.get(slot).secondary_words.clone();
    words.sort_by(|left, right| {
        let left_phase = ancient_words
            .get(&left.word_id)
            .map(|word| word.metadata().phase)
            .unwrap_or(u8::MAX);
        let right_phase = ancient_words
            .get(&right.word_id)
            .map(|word| word.metadata().phase)
            .unwrap_or(u8::MAX);
        left_phase
            .cmp(&right_phase)
            .then_with(|| left.word_id.as_str().cmp(right.word_id.as_str()))
    });

    apply_secondary_words(&mut blueprint, &words, known, ancient_words)?;

    let params = blueprint.params;
    Ok(SlotPreview {
        ability,
        blueprint,
        params,
    })
}

/// Resolves one armor ability using its independent Root Word inscription.
/// Armor has no weapon-style slot selection: the item’s first Primary ability
/// is the active ability for this initial cast API.
#[allow(clippy::too_many_arguments)]
pub fn resolve_armor_inscribed_ability(
    ability_id: &super::base_ability::AbilityId,
    inscription: Option<&ArmorInscription>,
    known: &KnownAncientLanguage,
    ability_registry: &BaseAbilityRegistry,
    root_words: &RootWordRegistry,
    ancient_words: &AncientWordRegistry,
    item: Option<&dyn Item>,
) -> Result<SlotPreview, CastBlockedReason> {
    let ability = ability_registry
        .get(ability_id)
        .ok_or(CastBlockedReason::MissingRegistryEntry)?;
    let Some(inscription) = inscription else {
        return Ok(SlotPreview {
            blueprint: item
                .map(|item| item.ability_blueprint(ability.as_ref()))
                .unwrap_or_else(|| ability.blueprint()),
            params: ability.base_params(),
            ability,
        });
    };
    let root_id = inscription
        .root_word
        .as_ref()
        .ok_or(CastBlockedReason::UnknownRootWord)?;
    if !known.knows_root_word(root_id) {
        return Err(CastBlockedReason::UnknownRootWord);
    }
    let mut blueprint = item
        .map(|item| item.ability_blueprint(ability.as_ref()))
        .unwrap_or_else(|| ability.blueprint());
    let root = root_words
        .get(root_id)
        .ok_or(CastBlockedReason::UnknownRootWord)?;
    let root_params = blueprint.params;
    root.apply_to_blueprint(&mut blueprint, &root_params);
    let mut words = inscription.secondary_words.clone();
    words.sort_by(|left, right| left.word_id.as_str().cmp(right.word_id.as_str()));
    apply_secondary_words(&mut blueprint, &words, known, ancient_words)?;
    let params = blueprint.params;
    Ok(SlotPreview {
        ability,
        blueprint,
        params,
    })
}

/// Executes a RootWord inscription using the same preview blueprint. This is
/// the additive cast path used once an item has migrated off the legacy model.
#[allow(clippy::too_many_arguments)]
pub fn cast_armor_inscribed_ability(
    ability_id: &super::base_ability::AbilityId,
    inscription: Option<&ArmorInscription>,
    known: &KnownAncientLanguage,
    ability_registry: &BaseAbilityRegistry,
    root_words: &RootWordRegistry,
    ancient_words: &AncientWordRegistry,
    ctx: &mut SpellCastContext,
    item: Option<&dyn Item>,
) -> Result<(), CastBlockedReason> {
    manifest_from_preview(
        resolve_armor_inscribed_ability(
            ability_id,
            inscription,
            known,
            ability_registry,
            root_words,
            ancient_words,
            item,
        ),
        ctx,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn cast_root_inscribed_slot(
    slot: AbilitySlot,
    abilities: &WeaponAbilities,
    selection: &AbilitySelection,
    inscription: &WeaponInscription,
    known: &KnownAncientLanguage,
    ability_registry: &BaseAbilityRegistry,
    root_words: &RootWordRegistry,
    ancient_words: &AncientWordRegistry,
    ctx: &mut SpellCastContext,
    item: Option<&dyn Item>,
) -> Result<(), CastBlockedReason> {
    manifest_from_preview(
        resolve_root_inscribed_slot(
            slot,
            abilities,
            selection,
            inscription,
            known,
            ability_registry,
            root_words,
            ancient_words,
            item,
        ),
        ctx,
    )
}

/// Lancia lo slot `slot` dell’arma incisa `inscriptions`.
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
    item: Option<&dyn Item>,
) -> Result<(), CastBlockedReason> {
    let SlotPreview {
        ability,
        blueprint,
        params,
    } = resolve_slot_preview(
        slot,
        abilities,
        selection,
        inscriptions,
        known,
        ability_registry,
        modifier_registry,
        item,
    )?;
    let inscription = inscriptions.get(slot);

    match &inscription.essence {
        Some(essence_id) => {
            let essence = essence_registry
                .get(essence_id)
                .ok_or(CastBlockedReason::MissingRegistryEntry)?;
            essence.manifest(ability.as_ref(), &params, ctx);
        }
        None => ability.default_manifestation(&params, ctx),
    }

    if matches!(
        blueprint.execution,
        super::blueprint::BlueprintExecution::Echo
    ) && blueprint.has_tag(AbilityTag::EchoCompatible)
    {
        match &inscription.essence {
            Some(essence_id) => {
                let essence = essence_registry
                    .get(essence_id)
                    .ok_or(CastBlockedReason::MissingRegistryEntry)?;
                essence.manifest(ability.as_ref(), &params, ctx);
            }
            None => ability.default_manifestation(&params, ctx),
        }
    }

    if let Some(word_id) = &inscription.ancient_word {
        let word = ancient_word_registry
            .get(word_id)
            .ok_or(CastBlockedReason::MissingRegistryEntry)?;
        word.post_process(ability.as_ref(), &params, ctx);
    }

    Ok(())
}
