//! Base-ability content and its registry.

pub mod arcane_orb;

use crate::abilities::BaseAbilityRegistry;

/// Builds the registry containing every base ability shipped by this game build.
pub fn default_base_abilities() -> BaseAbilityRegistry {
    let mut registry = BaseAbilityRegistry::default();
    arcane_orb::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_abilities_contains_only_arcane_orb() {
        assert_eq!(default_base_abilities().len(), 1);
    }
}
