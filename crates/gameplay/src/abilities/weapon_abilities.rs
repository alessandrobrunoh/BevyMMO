//! `AbilityLoadout` — le abilità offerte da un item equipaggiabile. Vive nel
//! catalogo e salva `AbilityId` (riferimenti al registry), non trait object.
//!
//! Ogni slot offre una o più abilità, inclusa l'Ultimate. La scelta attiva è
//! stato dell'esemplare (`AbilitySelection`), quindi armi e armature possono
//! condividere la stessa pipeline.

use serde::{Deserialize, Serialize};

use super::base_ability::AbilityId;
use super::slot::AbilitySlot;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityLoadout {
    pub primary: Vec<AbilityId>,
    pub secondary: Vec<AbilityId>,
    pub ultimate: Vec<AbilityId>,
}

impl AbilityLoadout {
    /// Builds a loadout, panicking with a clear message if any slot is empty.
    /// Prefer the `#[item(..., abilities(...))]` macro for content definitions,
    /// which validates the shape at compile time.
    pub fn new(
        primary: Vec<AbilityId>,
        secondary: Vec<AbilityId>,
        ultimate: Vec<AbilityId>,
    ) -> Self {
        assert!(
            !primary.is_empty(),
            "AbilityLoadout::primary must offer at least one ability"
        );
        assert!(
            !secondary.is_empty(),
            "AbilityLoadout::secondary must offer at least one ability"
        );
        assert!(
            !ultimate.is_empty(),
            "AbilityLoadout::ultimate must offer at least one ability"
        );
        Self {
            primary,
            secondary,
            ultimate,
        }
    }

    pub fn options_for(&self, slot: AbilitySlot) -> &[AbilityId] {
        match slot {
            AbilitySlot::Primary => &self.primary,
            AbilitySlot::Secondary => &self.secondary,
            AbilitySlot::Ultimate => &self.ultimate,
        }
    }
}

/// Compatibility alias while call sites migrate from the weapon-specific name.
pub type WeaponAbilities = AbilityLoadout;

/// The player's pick among each slot's offered abilities for one item
/// esemplare. Lives on `ItemInstance`, not the player.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AbilitySelection {
    pub primary: Option<AbilityId>,
    pub secondary: Option<AbilityId>,
    pub ultimate: Option<AbilityId>,
}

impl AbilitySelection {
    pub fn get(&self, slot: AbilitySlot) -> Option<&AbilityId> {
        match slot {
            AbilitySlot::Primary => self.primary.as_ref(),
            AbilitySlot::Secondary => self.secondary.as_ref(),
            AbilitySlot::Ultimate => self.ultimate.as_ref(),
        }
    }

    pub fn assign(&mut self, slot: AbilitySlot, ability: Option<AbilityId>) {
        match slot {
            AbilitySlot::Primary => self.primary = ability,
            AbilitySlot::Secondary => self.secondary = ability,
            AbilitySlot::Ultimate => self.ultimate = ability,
        }
    }
}

/// Resolves the gesture actually active for `slot`: the player's explicit
/// pick if they made one and it's still a valid option on this weapon,
/// otherwise the first offered option. Never returns `None` for a well-formed
/// `AbilityLoadout`, so a player who never opened the selection UI still gets
/// a sensible default.
pub fn resolve_active_ability<'a>(
    slot: AbilitySlot,
    abilities: &'a AbilityLoadout,
    selection: &'a AbilitySelection,
) -> Option<&'a AbilityId> {
    let options = abilities.options_for(slot);
    let picked = selection.get(slot).filter(|id| options.contains(id));
    picked.or_else(|| options.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AbilityLoadout {
        AbilityLoadout::new(
            vec![AbilityId::new("bolt"), AbilityId::new("spark")],
            vec![AbilityId::new("wave")],
            vec![AbilityId::new("convergence"), AbilityId::new("nova")],
        )
    }

    #[test]
    #[should_panic(expected = "AbilityLoadout::primary must offer at least one ability")]
    fn new_panics_on_empty_primary() {
        AbilityLoadout::new(
            vec![],
            vec![AbilityId::new("wave")],
            vec![AbilityId::new("convergence")],
        );
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
    fn ultimate_defaults_to_first_option() {
        let abilities = sample();
        let selection = AbilitySelection::default();
        assert_eq!(
            resolve_active_ability(AbilitySlot::Ultimate, &abilities, &selection),
            Some(&AbilityId::new("convergence"))
        );
    }

    #[test]
    fn ultimate_honors_an_explicit_valid_selection() {
        let abilities = sample();
        let mut selection = AbilitySelection::default();
        selection.assign(AbilitySlot::Ultimate, Some(AbilityId::new("nova")));
        assert_eq!(
            resolve_active_ability(AbilitySlot::Ultimate, &abilities, &selection),
            Some(&AbilityId::new("nova"))
        );
    }
}
