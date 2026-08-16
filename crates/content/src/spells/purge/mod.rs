//! Purge spell: removes purgeable buffs from the caster.

pub mod definition;

pub use definition::PurgeSpell;

use crate::spells::SpellRegistry;

pub fn register(registry: &mut SpellRegistry) {
    PurgeSpell::register(registry);
}
