//! Legacy spell content and its registry.

pub mod fireball;

use crate::spells::SpellRegistry;

/// Builds the registry containing every legacy spell shipped by this game build.
pub fn default_spells() -> SpellRegistry {
    let mut registry = SpellRegistry::default();
    fireball::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spells_contains_only_fireball() {
        assert_eq!(default_spells().len(), 1);
    }
}
