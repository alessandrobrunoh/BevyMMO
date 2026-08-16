//! Cleanse spell: removes removable debuffs from the caster.

pub mod definition;

pub use definition::CleanseSpell;

use crate::spells::SpellRegistry;

pub fn register(registry: &mut SpellRegistry) {
    CleanseSpell::register(registry);
}
