//! `WeaponAbilities` — i gesti che una variante d'arma offre. Vive nel
//! catalogo (`Item::weapon_abilities`), quindi salva `AbilityId` (riferimenti
//! al registry) e non `Arc<dyn BaseAbility>`, esattamente come `Equipment`
//! salva `ItemId` e non `Arc<dyn Item>`.
//!
//! Stessa forma di [`crate::items::SpellKit`], stesso vincolo: **Primary e
//! Secondary offrono 1+ gesti fra cui scegliere, Ultimate ne offre
//! esattamente 1**. La scelta del giocatore fra le opzioni di Primary/
//! Secondary è dato di gioco (per-esemplare, vedi [`super::inscription::AbilitySelection`]
//! su `ItemInstance`), non di catalogo: due Flame Staff possono avere
//! ciascuno un gesto Primary diverso attivo, pur offrendo lo stesso menu.

use serde::{Deserialize, Serialize};

use super::base_ability::AbilityId;
use super::slot::AbilitySlot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponAbilities {
    pub primary: Vec<AbilityId>,
    pub secondary: Vec<AbilityId>,
    pub ultimate: AbilityId,
}

impl WeaponAbilities {
    /// Builds a set of weapon abilities, panicking with a clear message if
    /// the Primary(1+)/Secondary(1+)/Ultimate(1) shape is violated. Prefer
    /// the `#[item(..., abilities(...))]` macro, which rejects an invalid
    /// shape at compile time instead of at startup; use this constructor
    /// directly only when abilities can't be expressed as a macro literal.
    pub fn new(primary: Vec<AbilityId>, secondary: Vec<AbilityId>, ultimate: AbilityId) -> Self {
        assert!(!primary.is_empty(), "WeaponAbilities::primary must offer at least one gesto");
        assert!(!secondary.is_empty(), "WeaponAbilities::secondary must offer at least one gesto");
        Self { primary, secondary, ultimate }
    }

    /// All gestures offered for `slot` — the menu the player picks from for
    /// Primary/Secondary, always a single-element slice for Ultimate.
    pub fn options_for(&self, slot: AbilitySlot) -> &[AbilityId] {
        match slot {
            AbilitySlot::Primary => &self.primary,
            AbilitySlot::Secondary => &self.secondary,
            AbilitySlot::Ultimate => std::slice::from_ref(&self.ultimate),
        }
    }
}

/// The player's pick among `WeaponAbilities::primary`/`secondary` for one
/// weapon esemplare. Lives on `ItemInstance` (not the player): swapping to a
/// different physical weapon of the same type keeps its own selection.
/// `Ultimate` never needs an entry — there is only ever one option.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AbilitySelection {
    pub primary: Option<AbilityId>,
    pub secondary: Option<AbilityId>,
}

impl AbilitySelection {
    pub fn get(&self, slot: AbilitySlot) -> Option<&AbilityId> {
        match slot {
            AbilitySlot::Primary => self.primary.as_ref(),
            AbilitySlot::Secondary => self.secondary.as_ref(),
            AbilitySlot::Ultimate => None,
        }
    }

    pub fn assign(&mut self, slot: AbilitySlot, ability: Option<AbilityId>) {
        match slot {
            AbilitySlot::Primary => self.primary = ability,
            AbilitySlot::Secondary => self.secondary = ability,
            AbilitySlot::Ultimate => {}
        }
    }
}

/// Resolves the gesture actually active for `slot`: the player's explicit
/// pick if they made one and it's still a valid option on this weapon,
/// otherwise the first offered option — Ultimate always resolves to its one
/// and only gesture. Never returns `None` given a well-formed
/// `WeaponAbilities` (Primary/Secondary always have >= 1 option), so a
/// player who never opened the selection UI still gets a sensible default.
pub fn resolve_active_ability<'a>(
    slot: AbilitySlot,
    abilities: &'a WeaponAbilities,
    selection: &'a AbilitySelection,
) -> Option<&'a AbilityId> {
    let options = abilities.options_for(slot);
    let picked = selection.get(slot).filter(|id| options.contains(id));
    picked.or_else(|| options.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> WeaponAbilities {
        WeaponAbilities::new(
            vec![AbilityId::new("bolt"), AbilityId::new("spark")],
            vec![AbilityId::new("wave")],
            AbilityId::new("convergence"),
        )
    }

    #[test]
    #[should_panic(expected = "primary must offer at least one gesto")]
    fn new_panics_on_empty_primary() {
        WeaponAbilities::new(vec![], vec![AbilityId::new("wave")], AbilityId::new("convergence"));
    }

    #[test]
    fn resolve_defaults_to_the_first_option_when_nothing_selected() {
        let abilities = sample();
        let selection = AbilitySelection::default();
        assert_eq!(
            resolve_active_ability(AbilitySlot::Primary, &abilities, &selection),
            Some(&AbilityId::new("bolt"))
        );
    }

    #[test]
    fn resolve_honors_an_explicit_valid_selection() {
        let abilities = sample();
        let mut selection = AbilitySelection::default();
        selection.assign(AbilitySlot::Primary, Some(AbilityId::new("spark")));
        assert_eq!(
            resolve_active_ability(AbilitySlot::Primary, &abilities, &selection),
            Some(&AbilityId::new("spark"))
        );
    }

    #[test]
    fn resolve_falls_back_when_the_selection_is_no_longer_a_valid_option() {
        let abilities = sample();
        let mut selection = AbilitySelection::default();
        selection.assign(AbilitySlot::Primary, Some(AbilityId::new("not_offered")));
        assert_eq!(
            resolve_active_ability(AbilitySlot::Primary, &abilities, &selection),
            Some(&AbilityId::new("bolt"))
        );
    }

    #[test]
    fn ultimate_always_resolves_to_its_single_gesture() {
        let abilities = sample();
        let selection = AbilitySelection::default();
        assert_eq!(
            resolve_active_ability(AbilitySlot::Ultimate, &abilities, &selection),
            Some(&AbilityId::new("convergence"))
        );
    }
}
