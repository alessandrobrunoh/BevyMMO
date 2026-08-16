//! Fireball spell.

pub mod definition;

pub use definition::FireballSpell;

use crate::spells::SpellRegistry;

/// Adds this content package to the legacy spell registry.
pub fn register(registry: &mut SpellRegistry) {
    FireballSpell::register(registry);
}
