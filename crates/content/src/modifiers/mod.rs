//! Modifier content and its registry.

pub mod espandere;

use crate::abilities::ModifierRegistry;

/// Builds the registry containing every modifier shipped by this game build.
pub fn default_modifiers() -> ModifierRegistry {
    let mut registry = ModifierRegistry::default();
    espandere::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_modifiers_contains_only_espandere() {
        assert_eq!(default_modifiers().len(), 1);
    }
}
