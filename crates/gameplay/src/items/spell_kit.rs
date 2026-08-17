//! Optional "spell kit" carried by an item: which spells it makes available
//! on the Q/W/E hotbar slots while equipped.
//!
//! This is the data half of the item → spell pipeline that replaces the old
//! free-form Spellbook: an item no longer just grants stat bonuses
//! ([`super::effects::ItemEffect`]), it can *also* offer a pool of spells the
//! player may pick from for Q, for W, and for E. The player's actual pick
//! still lives in [`crate::spells::components::SpellHotbar`] (unchanged);
//! what changes is where the pool of legal picks comes from — see
//! `bevymmo_server::items::available_spells` for the reactive system that
//! unions the kits of every equipped item into that pool.
//!
//! [`crate::spells::components::HotbarSlot`] is reused here (instead of a new
//! enum) so there is a single definition of "which key" throughout the
//! codebase.

use crate::spells::components::HotbarSlot;
use crate::spells::registry::SpellId;

/// Spells an item grants access to while equipped.
///
/// Invariants — enforced at compile time by the `#[item(...)]` macro
/// (`bevymmo_props_macro::item`) for generated items, and at construction
/// time by [`SpellKit::new`] for hand-built ones:
/// - `q` has at least one candidate.
/// - `w` has at least one candidate.
/// - `e` is exactly one spell — there is no "empty E" and no "list of E".
///
/// A player only ever has *one* active spell per key at a time (stored in
/// `SpellHotbar`); a `SpellKit` describes the *menu* an equipped item offers
/// for that key, not what fires when the key is pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellKit {
    pub q: Vec<SpellId>,
    pub w: Vec<SpellId>,
    pub e: SpellId,
}

impl SpellKit {
    /// Builds a kit, panicking with a clear message if the Q/W/E shape is
    /// violated. Prefer the `#[item(...)]` macro, which rejects an invalid
    /// shape at compile time instead of at startup; use this constructor
    /// directly only for items whose kit can't be expressed as a macro
    /// literal (e.g. built from data loaded at runtime).
    pub fn new(q: Vec<SpellId>, w: Vec<SpellId>, e: SpellId) -> Self {
        assert!(
            !q.is_empty(),
            "SpellKit::q must have at least one spell (an item that grants Q spells must offer at least one option)"
        );
        assert!(
            !w.is_empty(),
            "SpellKit::w must have at least one spell (an item that grants W spells must offer at least one option)"
        );
        Self { q, w, e }
    }

    /// Candidate spells offered for a given hotbar key.
    ///
    /// `HotbarSlot::E` always yields a one-element slice — kept as a slice
    /// (rather than returning `&SpellId` for E specially) so callers that
    /// union kits from several equipped items can treat all three keys
    /// uniformly.
    pub fn candidates_for(&self, slot: HotbarSlot) -> &[SpellId] {
        match slot {
            HotbarSlot::Q => &self.q,
            HotbarSlot::W => &self.w,
            HotbarSlot::E => std::slice::from_ref(&self.e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_for_e_is_a_single_element_slice() {
        let kit = SpellKit::new(
            vec![SpellId::new("attack")],
            vec![SpellId::new("fireball")],
            SpellId::new("meteorite"),
        );
        assert_eq!(
            kit.candidates_for(HotbarSlot::E),
            &[SpellId::new("meteorite")]
        );
    }

    #[test]
    fn candidates_for_q_returns_all_options() {
        let kit = SpellKit::new(
            vec![SpellId::new("attack"), SpellId::new("fireball")],
            vec![SpellId::new("stun_field")],
            SpellId::new("meteorite"),
        );
        assert_eq!(kit.candidates_for(HotbarSlot::Q).len(), 2);
    }

    #[test]
    #[should_panic(expected = "SpellKit::q must have at least one spell")]
    fn new_panics_on_empty_q() {
        SpellKit::new(
            vec![],
            vec![SpellId::new("fireball")],
            SpellId::new("meteorite"),
        );
    }

    #[test]
    #[should_panic(expected = "SpellKit::w must have at least one spell")]
    fn new_panics_on_empty_w() {
        SpellKit::new(
            vec![SpellId::new("attack")],
            vec![],
            SpellId::new("meteorite"),
        );
    }
}
