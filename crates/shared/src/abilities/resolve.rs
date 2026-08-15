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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    use super::super::base_ability::{AbilityId, BaseAbility};
    use super::super::essence::EssenceId;
    use super::super::inscription::Inscription;
    use crate::base_abilities_impl::arcane_orb::ArcaneOrb;
    use crate::essences_impl::fuoco::FuocoEssence;
    use crate::stats::components::CombatStats;

    /// Un bersaglio piazzato dritto davanti al lanciatore, che guarda +Z:
    /// l'aggancio frontale di un gesto `Projectile` lo trova senza che
    /// nessuno lo abbia selezionato.
    const TARGET_IN_FRONT: Vec3 = Vec3::new(0.0, 0.0, 5.0);

    fn registries() -> (BaseAbilityRegistry, EssenceRegistry, ModifierRegistry, AncientWordRegistry) {
        let mut abilities = BaseAbilityRegistry::default();
        ArcaneOrb::register(&mut abilities);
        let mut essences = EssenceRegistry::default();
        FuocoEssence::register(&mut essences);
        (abilities, essences, ModifierRegistry::default(), AncientWordRegistry::default())
    }

    fn arcane_orb_everywhere() -> WeaponAbilities {
        WeaponAbilities::new(
            vec![AbilityId::new(ArcaneOrb::ID)],
            vec![AbilityId::new(ArcaneOrb::ID)],
            AbilityId::new(ArcaneOrb::ID),
        )
    }

    #[test]
    fn casts_and_emits_boosted_fire_damage_when_the_essence_is_known() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let target = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let potential = vec![(target, TARGET_IN_FRONT)];
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, Some(target), &potential);

        let (abilities, essences, modifiers, words) = registries();
        let weapon = arcane_orb_everywhere();
        let selection = AbilitySelection::default();

        let inscriptions = WeaponInscriptions {
            primary: Inscription {
                essence: Some(EssenceId::new("fuoco")),
                modifiers: vec![],
                ancient_word: None,
            },
            ..Default::default()
        };

        let mut known = KnownGlyphs::default();
        known.essences.insert(EssenceId::new("fuoco"));

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &weapon,
            &selection,
            &inscriptions,
            &known,
            &abilities,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert!(result.is_ok());
        // Il gesto lancia una palla: il danno viaggia con lei, non parte
        // istantaneo dal lanciatore.
        assert_eq!(ctx.pending_projectiles.len(), 1);
        assert_eq!(ctx.pending_projectiles[0].target, target);
        let expected = ArcaneOrb.base_params().power * FuocoEssence::POWER_MULTIPLIER;
        assert!((ctx.pending_projectiles[0].damage - expected).abs() < 0.01);
    }

    #[test]
    fn blocks_the_whole_slot_when_the_inscribed_essence_is_unknown() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        let (abilities, essences, modifiers, words) = registries();
        let weapon = arcane_orb_everywhere();
        let selection = AbilitySelection::default();

        let inscriptions = WeaponInscriptions {
            primary: Inscription {
                essence: Some(EssenceId::new("fuoco")),
                modifiers: vec![],
                ancient_word: None,
            },
            ..Default::default()
        };

        // Il caster non ha "fuoco" nel proprio Vocabolario.
        let known = KnownGlyphs::default();

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &weapon,
            &selection,
            &inscriptions,
            &known,
            &abilities,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert_eq!(result, Err(CastBlockedReason::UnknownGlyph));
        assert!(ctx.pending_projectiles.is_empty());
        assert!(ctx.pending_damage.is_empty());
    }

    #[test]
    fn falls_back_to_default_manifestation_when_no_essence_is_inscribed() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let target = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let potential = vec![(target, TARGET_IN_FRONT)];
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, Some(target), &potential);

        let (abilities, essences, modifiers, words) = registries();
        let weapon = arcane_orb_everywhere();
        let selection = AbilitySelection::default();
        let inscriptions = WeaponInscriptions::default(); // nessuna incisione
        let known = KnownGlyphs::default();

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &weapon,
            &selection,
            &inscriptions,
            &known,
            &abilities,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert!(result.is_ok());
        assert_eq!(ctx.pending_projectiles.len(), 1);
        // Nessuna Essenza incisa: potenza base, nessun moltiplicatore.
        assert_eq!(ctx.pending_projectiles[0].damage, ArcaneOrb.base_params().power);
    }

    #[test]
    fn aims_a_projectile_at_the_first_entity_in_front_without_any_selection() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let near = world.spawn_empty().id();
        let far = world.spawn_empty().id();
        let behind = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let potential = vec![
            (far, Vec3::new(0.0, 0.0, 9.0)),
            (behind, Vec3::new(0.0, 0.0, -4.0)),
            (near, Vec3::new(0.0, 0.0, 3.0)),
        ];
        // Nessun bersaglio selezionato: si spara dove si guarda.
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &potential);

        let (abilities, essences, modifiers, words) = registries();

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &arcane_orb_everywhere(),
            &AbilitySelection::default(),
            &WeaponInscriptions::default(),
            &KnownGlyphs::default(),
            &abilities,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert!(result.is_ok());
        assert_eq!(ctx.pending_projectiles.len(), 1);
        assert_eq!(ctx.pending_projectiles[0].target, near);
    }

    #[test]
    fn resolves_the_players_selected_gesture_among_multiple_primary_options() {
        use crate::base_abilities_impl::arcane_gale::ArcaneGale;

        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let mut ctx = SpellCastContext::new(caster, Vec3::new(5.0, 0.0, 0.0), &combat, Vec3::Z, None, None, &[]);

        let (mut abilities_registry, essences, modifiers, words) = registries();
        ArcaneGale::register(&mut abilities_registry);

        let weapon = WeaponAbilities::new(
            vec![AbilityId::new(ArcaneOrb::ID), AbilityId::new(ArcaneGale::ID)],
            vec![AbilityId::new(ArcaneOrb::ID)],
            AbilityId::new(ArcaneOrb::ID),
        );
        let mut selection = AbilitySelection::default();
        selection.assign(AbilitySlot::Primary, Some(AbilityId::new(ArcaneGale::ID)));

        let inscriptions = WeaponInscriptions::default();
        let known = KnownGlyphs::default();

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &weapon,
            &selection,
            &inscriptions,
            &known,
            &abilities_registry,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert!(result.is_ok());
        // ArcaneGale è un cono ad area: la manifestazione di default emette
        // un'AoE, non una palla — prova che il gesto RISOLTO è davvero
        // ArcaneGale (scelto), non il primo AbilityId della lista.
        assert_eq!(ctx.pending_aoes.len(), 1);
        assert!(ctx.pending_projectiles.is_empty());
    }

    #[test]
    fn a_controlling_gesture_lays_both_a_damage_and_a_stun_region_after_its_warning() {
        use crate::base_abilities_impl::binding_seal::BindingSeal;

        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let aimed_at = Vec3::new(0.0, 0.0, 6.0);
        let mut ctx =
            SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, Some(aimed_at), None, &[]);

        let (mut abilities_registry, essences, modifiers, words) = registries();
        BindingSeal::register(&mut abilities_registry);

        let weapon = WeaponAbilities::new(
            vec![AbilityId::new(ArcaneOrb::ID)],
            vec![AbilityId::new(BindingSeal::ID)],
            AbilityId::new(ArcaneOrb::ID),
        );

        let result = cast_inscribed_slot(
            AbilitySlot::Secondary,
            &weapon,
            &AbilitySelection::default(),
            &WeaponInscriptions::default(),
            &KnownGlyphs::default(),
            &abilities_registry,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert!(result.is_ok());
        assert_eq!(ctx.pending_aoes.len(), 2);
        // Entrambe le regioni scattano allo scadere dello stesso preavviso,
        // sul punto mirato: il cerchio che si vede a terra è esattamente
        // quello che poi colpisce.
        for aoe in &ctx.pending_aoes {
            assert_eq!(aoe.center, aimed_at);
            assert_eq!(aoe.initial_delay_seconds, BindingSeal.impact_delay());
        }
        assert!(ctx.pending_aoes.iter().any(|aoe| matches!(
            aoe.effect,
            crate::spells::context::AoeEffect::CrowdControl { .. }
        )));
        assert!(ctx
            .pending_aoes
            .iter()
            .any(|aoe| matches!(aoe.effect, crate::spells::context::AoeEffect::Damage { .. })));
    }

    #[test]
    fn a_ground_gesture_cannot_be_aimed_past_its_range() {
        use crate::base_abilities_impl::arcane_seal::ArcaneSeal;

        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        // Puntato molto oltre la gittata del gesto.
        let mut ctx = SpellCastContext::new(
            caster,
            Vec3::ZERO,
            &combat,
            Vec3::Z,
            Some(Vec3::new(0.0, 0.0, 100.0)),
            None,
            &[],
        );

        let (mut abilities_registry, essences, modifiers, words) = registries();
        ArcaneSeal::register(&mut abilities_registry);

        let weapon = WeaponAbilities::new(
            vec![AbilityId::new(ArcaneSeal::ID)],
            vec![AbilityId::new(ArcaneOrb::ID)],
            AbilityId::new(ArcaneOrb::ID),
        );

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &weapon,
            &AbilitySelection::default(),
            &WeaponInscriptions::default(),
            &KnownGlyphs::default(),
            &abilities_registry,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert!(result.is_ok());
        assert_eq!(ctx.pending_aoes.len(), 1);
        let range = ArcaneSeal.base_params().range;
        assert!((ctx.pending_aoes[0].center.length() - range).abs() < 0.001);
    }
}
