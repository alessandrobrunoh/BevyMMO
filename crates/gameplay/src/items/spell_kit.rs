//! Optional "spell kit" carried by an item: which spells it makes available
//! on the Primary/Secondary/Ultimate hotbar slots while equipped.
//!
//! This is the data half of the item → spell pipeline that replaces the old
//! free-form Spellbook: an item no longer just grants stat bonuses
//! ([`super::effects::ItemEffect`]), it can *also* offer a pool of spells the
//! player may pick from for each [`crate::abilities::AbilitySlot`]. The
//! player's actual pick still lives in
//! [`crate::spells::components::SpellHotbar`]; what changes is where the pool
//! of legal picks comes from — see
//! `bevymmo_server::items::available_spells` for the reactive system that
//! unions the kits of every equipped item into that pool.

use crate::abilities::AbilitySlot;
use crate::spells::registry::SpellId;

/// Spells an item grants access to while equipped.
///
/// Invariants — enforced at compile time by the `#[item(...)]` macro
/// (`bevymmo_props_macro::item`) for generated items, and at construction
/// time by [`SpellKit::new`] for hand-built ones:
/// - `primary` has at least one candidate.
/// - `secondary` has at least one candidate.
/// - `ultimate` is exactly one spell.
///
/// A player only ever has *one* active spell per slot at a time (stored in
/// `SpellHotbar`); a `SpellKit` describes the *menu* an equipped item offers
/// for that slot, not what fires when the key is pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellKit {
    pub primary: Vec<SpellId>,
    pub secondary: Vec<SpellId>,
    pub ultimate: SpellId,
}

impl SpellKit {
    /// Builds a kit, panicking with a clear message if the slot shape is
    /// violated. Prefer the `#[item(...)]` macro, which rejects an invalid
    /// shape at compile time instead of at startup; use this constructor
    /// directly only for items whose kit can't be expressed as a macro
    /// literal (e.g. built from data loaded at runtime).
    pub fn new(primary: Vec<SpellId>, secondary: Vec<SpellId>, ultimate: SpellId) -> Self {
        assert!(
            !primary.is_empty(),
            "SpellKit::primary must have at least one spell (an item that grants primary spells must offer at least one option)"
        );
        assert!(
            !secondary.is_empty(),
            "SpellKit::secondary must have at least one spell (an item that grants secondary spells must offer at least one option)"
        );
        Self {
            primary,
            secondary,
            ultimate,
        }
    }

    /// Candidate spells offered for a given ability slot.
    ///
    /// [`AbilitySlot::Ultimate`] always yields a one-element slice — kept as a
    /// slice (rather than returning `&SpellId` for Ultimate specially) so
    /// callers that union kits from several equipped items can treat all
    /// three slots uniformly.
    pub fn candidates_for(&self, slot: AbilitySlot) -> &[SpellId] {
        match slot {
            AbilitySlot::Primary => &self.primary,
            AbilitySlot::Secondary => &self.secondary,
            AbilitySlot::Ultimate => std::slice::from_ref(&self.ultimate),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_for_ultimate_is_a_single_element_slice() {
        let kit = SpellKit::new(
            vec![SpellId::new("attack")],
            vec![SpellId::new("fireball")],
            SpellId::new("meteorite"),
        );
        assert_eq!(
            kit.candidates_for(AbilitySlot::Ultimate),
            &[SpellId::new("meteorite")]
        );
    }

    #[test]
    fn candidates_for_primary_returns_all_options() {
        let kit = SpellKit::new(
            vec![SpellId::new("attack"), SpellId::new("fireball")],
            vec![SpellId::new("stun_field")],
            SpellId::new("meteorite"),
        );
        assert_eq!(kit.candidates_for(AbilitySlot::Primary).len(), 2);
    }

    #[test]
    #[should_panic(expected = "SpellKit::primary must have at least one spell")]
    fn new_panics_on_empty_primary() {
        SpellKit::new(
            vec![],
            vec![SpellId::new("fireball")],
            SpellId::new("meteorite"),
        );
    }

    #[test]
    #[should_panic(expected = "SpellKit::secondary must have at least one spell")]
    fn new_panics_on_empty_secondary() {
        SpellKit::new(
            vec![SpellId::new("attack")],
            vec![],
            SpellId::new("meteorite"),
        );
    }
}
