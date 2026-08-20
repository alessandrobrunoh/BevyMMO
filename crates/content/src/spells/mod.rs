//! NPC-only spell content.
//!
//! Player abilities use the Eidolon Root Word pipeline. Fireball remains here
//! solely because enemy AI uses the generic spell runtime for its attack.

pub mod boss;
pub mod fireball;

use crate::spells::SpellRegistry;

pub fn default_spells() -> SpellRegistry {
    let mut registry = SpellRegistry::default();
    fireball::register(&mut registry);
    boss::register(&mut registry);
    registry
}
