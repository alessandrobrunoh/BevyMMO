//! Legacy spell content and its registry.

pub mod cleanse;
pub mod fireball;
pub mod purge;

use crate::spells::SpellRegistry;

/// Builds the registry containing every legacy spell shipped by this game build.
pub fn default_spells() -> SpellRegistry {
    let mut registry = SpellRegistry::default();
    cleanse::register(&mut registry);
    fireball::register(&mut registry);
    purge::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spells_contains_core_effect_spells() {
        let registry = default_spells();

        assert_eq!(registry.len(), 3);
        assert!(registry.get(&"cleanse".into()).is_some());
        assert!(registry.get(&"fireball".into()).is_some());
        assert!(registry.get(&"purge".into()).is_some());
    }
}
