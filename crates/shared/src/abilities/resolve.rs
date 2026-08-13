//! L'interprete: `BaseAbility` (gesto) + `Inscription` (Glifi) →
//! effetto finale, tramite lo stesso `SpellCastContext` già usato dalle
//! spell "classiche" — riusa tutta la pipeline di cast/rete esistente.

use super::ancient_word::AncientWordRegistry;
use super::base_ability::{AbilityParams, BaseAbilityRegistry};
use super::essence::EssenceRegistry;
use super::inscription::WeaponInscriptions;
use super::known_glyphs::KnownGlyphs;
use super::modifier::ModifierRegistry;
use super::slot::AbilitySlot;
use super::weapon_abilities::WeaponAbilities;
use crate::spells::context::SpellCastContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastBlockedReason {
    /// Lo slot è incisione con almeno un Glifo che il caster non conosce:
    /// blocco totale, come deciso — nessun cast, non solo "senza effetto".
    UnknownGlyph,
    /// L'`AbilityId`/`EssenceId`/... incisi non esistono più nei registry
    /// (dati incoerenti — non dovrebbe succedere con contenuti registrati
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
fn resolve_params(
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

/// Lancia lo slot `slot` dell'arma incisa `inscriptions`.
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
    inscriptions: &WeaponInscriptions,
    known: &KnownGlyphs,
    ability_registry: &BaseAbilityRegistry,
    essence_registry: &EssenceRegistry,
    modifier_registry: &ModifierRegistry,
    ancient_word_registry: &AncientWordRegistry,
    ctx: &mut SpellCastContext,
) -> Result<(), CastBlockedReason> {
    let inscription = inscriptions.get(slot);
    if !known.fully_knows(inscription) {
        return Err(CastBlockedReason::UnknownGlyph);
    }

    let ability_id = abilities.get(slot);
    let ability = ability_registry.get(ability_id).ok_or(CastBlockedReason::MissingRegistryEntry)?;

    let params = resolve_params(ability.base_params(), &inscription.modifiers, modifier_registry);

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
    use crate::base_abilities_impl::staff_bolt::StaffBolt;
    use crate::essences_impl::fuoco::FuocoEssence;
    use crate::stats::components::CombatStats;

    fn registries() -> (BaseAbilityRegistry, EssenceRegistry, ModifierRegistry, AncientWordRegistry) {
        let mut abilities = BaseAbilityRegistry::default();
        StaffBolt::register(&mut abilities);
        let mut essences = EssenceRegistry::default();
        FuocoEssence::register(&mut essences);
        (abilities, essences, ModifierRegistry::default(), AncientWordRegistry::default())
    }

    fn staff_bolt_everywhere() -> WeaponAbilities {
        WeaponAbilities {
            primary: AbilityId::new(StaffBolt::ID),
            secondary: AbilityId::new(StaffBolt::ID),
            ultimate: AbilityId::new(StaffBolt::ID),
        }
    }

    #[test]
    fn casts_and_emits_boosted_fire_damage_when_the_essence_is_known() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let target = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let potential = vec![(target, Vec3::ZERO)];
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, Some(target), &potential);

        let (abilities, essences, modifiers, words) = registries();
        let weapon = staff_bolt_everywhere();

        let inscriptions = WeaponInscriptions {
            primary: Inscription { essence: Some(EssenceId::new("fuoco")), modifiers: vec![], ancient_word: None },
            ..Default::default()
        };

        let mut known = KnownGlyphs::default();
        known.essences.insert(EssenceId::new("fuoco"));

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &weapon,
            &inscriptions,
            &known,
            &abilities,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert!(result.is_ok());
        assert_eq!(ctx.pending_damage.len(), 1);
        assert_eq!(ctx.pending_damage[0].target, target);
        let expected = StaffBolt.base_params().power * FuocoEssence::POWER_MULTIPLIER;
        assert!((ctx.pending_damage[0].amount - expected).abs() < 0.01);
    }

    #[test]
    fn blocks_the_whole_slot_when_the_inscribed_essence_is_unknown() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        let (abilities, essences, modifiers, words) = registries();
        let weapon = staff_bolt_everywhere();

        let inscriptions = WeaponInscriptions {
            primary: Inscription { essence: Some(EssenceId::new("fuoco")), modifiers: vec![], ancient_word: None },
            ..Default::default()
        };

        // Il caster non ha "fuoco" nel proprio Vocabolario.
        let known = KnownGlyphs::default();

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &weapon,
            &inscriptions,
            &known,
            &abilities,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert_eq!(result, Err(CastBlockedReason::UnknownGlyph));
        assert!(ctx.pending_damage.is_empty());
    }

    #[test]
    fn falls_back_to_default_manifestation_when_no_essence_is_inscribed() {
        let mut world = World::new();
        let caster = world.spawn_empty().id();
        let target = world.spawn_empty().id();
        let combat = CombatStats { attack_power: 0.0, armor: 0.0 };
        let potential = vec![(target, Vec3::ZERO)];
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, Some(target), &potential);

        let (abilities, essences, modifiers, words) = registries();
        let weapon = staff_bolt_everywhere();
        let inscriptions = WeaponInscriptions::default(); // nessuna incisione
        let known = KnownGlyphs::default();

        let result = cast_inscribed_slot(
            AbilitySlot::Primary,
            &weapon,
            &inscriptions,
            &known,
            &abilities,
            &essences,
            &modifiers,
            &words,
            &mut ctx,
        );

        assert!(result.is_ok());
        assert_eq!(ctx.pending_damage.len(), 1);
        // Nessuna Essenza incisa: potenza base, nessun moltiplicatore.
        assert_eq!(ctx.pending_damage[0].amount, StaffBolt.base_params().power);
    }
}
