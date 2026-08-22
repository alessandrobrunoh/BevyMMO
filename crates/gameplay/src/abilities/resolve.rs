//! L'interprete: `BaseAbility` (gesto) + `Inscription` (Glifi) →
//! effetto finale, tramite lo stesso `SpellCastContext` già usato dalle
//! spell "classiche" — riusa tutta la pipeline di cast/rete esistente.

use super::ancient_word::AncientWordRegistry;
use super::base_ability::{
    AbilityId, AbilityParams, AbilityTag, ArcBaseAbility, BaseAbilityRegistry,
};
use super::blueprint::AbilityBlueprint;
use super::inscription::{ArmorInscription, KitInscription, WeaponInscription};
use super::known_glyphs::KnownAncientLanguage;
use super::root_word::RootWordRegistry;
use super::slot::AbilitySlot;
use super::weapon_abilities::{resolve_active_ability, AbilitySelection, WeaponAbilities};
use crate::items::Item;
use crate::spells::context::SpellCastContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastBlockedReason {
    /// L'`AbilityId`/`EssenceId`/... incisi non esistono più nei registry,
    /// o `WeaponAbilities` non offre nessun gesto per lo slot (dati
    /// incoerenti — non dovrebbe succedere con contenuti registrati
    /// correttamente, ma viene gestito senza panic).
    MissingRegistryEntry,
    UnknownRootWord,
    UnknownAncientWord,
    IncompatibleAncientWord,
}

/// Il gesto attivo su uno slot e i suoi parametri già modificati — tutto ciò
/// che serve per sapere COSA e DOVE colpirà, senza lanciarlo.
pub struct SlotPreview {
    pub ability: ArcBaseAbility,
    pub blueprint: AbilityBlueprint,
    /// Compatibility view of `blueprint.params` for existing callers.
    pub params: AbilityParams,
}

/// Manifests an already-resolved preview into `ctx`. Shared by the player
/// wrappers and by AI catalog casts: geometry and payload live on the
/// blueprint, not on who asked for the cast.
pub fn cast_ability_preview(preview: &SlotPreview, ctx: &mut SpellCastContext) {
    preview.ability.manifest_blueprint(&preview.blueprint, ctx);
    if preview.blueprint.echo && preview.blueprint.has_tag(AbilityTag::EchoCompatible) {
        // Echo is a second manifestation of the already-resolved blueprint;
        // it never re-enters item/root/word resolution, so recursive echoes
        // are impossible.
        preview.ability.manifest_blueprint(&preview.blueprint, ctx);
    }
}

fn manifest_blueprint(preview: &SlotPreview, ctx: &mut SpellCastContext) {
    cast_ability_preview(preview, ctx);
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
    }
    apply_secondary_words_trusted(blueprint, words, ancient_words)
}

/// Applies secondary words without a known-glyph gate. Catalog kits are
/// trusted: the author listed the word, so the caster "knows" it.
fn apply_secondary_words_trusted(
    blueprint: &mut AbilityBlueprint,
    words: &[super::inscription::SecondaryWord],
    ancient_words: &AncientWordRegistry,
) -> Result<(), CastBlockedReason> {
    for secondary in words {
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

/// Resolves a `BaseAbility` for any caster (player, enemy, NPC).
///
/// No item and no known-glyph check: an enemy kit that names Flame on Cleave
/// is allowed to burn. Player wrappers (`resolve_root_inscribed_slot`,
/// `resolve_armor_inscribed_ability`) still enforce equipment and glyphs
/// before they reach this, or they keep their own path until they wrap it.
pub fn resolve_ability(
    ability_id: &AbilityId,
    inscription: Option<&KitInscription>,
    ability_registry: &BaseAbilityRegistry,
    root_words: &RootWordRegistry,
    ancient_words: &AncientWordRegistry,
) -> Result<SlotPreview, CastBlockedReason> {
    let ability = ability_registry
        .get(ability_id)
        .ok_or(CastBlockedReason::MissingRegistryEntry)?;
    let mut blueprint = ability.blueprint();

    if let Some(inscription) = inscription {
        if let Some(root_id) = inscription.root_word.as_ref() {
            let root = root_words
                .get(root_id)
                .ok_or(CastBlockedReason::UnknownRootWord)?;
            let root_params = blueprint.params;
            root.apply_to_blueprint(&mut blueprint, &root_params);
        }
        apply_secondary_words_trusted(&mut blueprint, &inscription.secondary_words, ancient_words)?;
    }

    let params = blueprint.params;
    Ok(SlotPreview {
        ability,
        blueprint,
        params,
    })
}

/// Resolves the active ability through the RootWord inscription pipeline.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::base_ability::{AbilityGeometry, AbilityTag, BaseAbility};
    use crate::abilities::blueprint::ManifestationKind;
    use std::sync::Arc;

    struct Slash;
    impl BaseAbility for Slash {
        fn id(&self) -> AbilityId {
            AbilityId::new("slash")
        }
        fn display_name(&self) -> &'static str {
            "Slash"
        }
        fn tags(&self) -> &'static [AbilityTag] {
            &[AbilityTag::Melee, AbilityTag::Area]
        }
        fn geometry(&self) -> AbilityGeometry {
            AbilityGeometry::Cone {
                radius: 5.0,
                angle_deg: 85.0,
            }
        }
        fn base_params(&self) -> AbilityParams {
            AbilityParams {
                potency: 115.0,
                area: 5.0,
                range: 5.0,
                cast_time: 0.25,
                cooldown: 3.0,
                mana_cost: 9.0,
            }
        }
        fn animation(&self) -> &'static str {
            "slash"
        }
        fn impact_vfx(&self) -> &'static str {
            "slash_impact"
        }
    }

    fn registries() -> (BaseAbilityRegistry, RootWordRegistry, AncientWordRegistry) {
        let mut abilities = BaseAbilityRegistry::default();
        abilities.register(Arc::new(Slash));
        (
            abilities,
            RootWordRegistry::default(),
            AncientWordRegistry::default(),
        )
    }

    #[test]
    fn missing_ability_is_a_registry_miss() {
        let (abilities, roots, words) = registries();
        let err = resolve_ability(&AbilityId::new("nope"), None, &abilities, &roots, &words);
        assert!(matches!(err, Err(CastBlockedReason::MissingRegistryEntry)));
    }

    #[test]
    fn naked_kit_keeps_the_gesture_blueprint() {
        let (abilities, roots, words) = registries();
        let preview = resolve_ability(&AbilityId::new("slash"), None, &abilities, &roots, &words)
            .expect("slash is registered");
        assert_eq!(preview.ability.id().as_str(), "slash");
        assert_eq!(preview.params.potency, 115.0);
        assert_eq!(preview.params.range, 5.0);
        assert_eq!(preview.blueprint.payload.kind, ManifestationKind::Damage);
        assert!(preview.blueprint.payload.status_ids.is_empty());
    }

    #[test]
    fn empty_inscription_is_the_naked_gesture() {
        let (abilities, roots, words) = registries();
        let empty = KitInscription::default();
        let preview = resolve_ability(
            &AbilityId::new("slash"),
            Some(&empty),
            &abilities,
            &roots,
            &words,
        )
        .expect("empty kit is legal");
        assert_eq!(preview.params.potency, 115.0);
        assert_eq!(preview.blueprint.payload.kind, ManifestationKind::Damage);
    }

    #[test]
    fn unknown_root_word_on_a_kit_is_blocked() {
        let (abilities, roots, words) = registries();
        let inscription = KitInscription {
            root_word: Some(crate::abilities::RootWordId::new("flame")),
            secondary_words: Vec::new(),
        };
        let err = resolve_ability(
            &AbilityId::new("slash"),
            Some(&inscription),
            &abilities,
            &roots,
            &words,
        );
        assert!(matches!(err, Err(CastBlockedReason::UnknownRootWord)));
    }
}
