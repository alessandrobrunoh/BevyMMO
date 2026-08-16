//! Essence content and its registry.

pub mod fuoco;

use crate::abilities::EssenceRegistry;

/// Builds the registry containing every essence shipped by this game build.
pub fn default_essences() -> EssenceRegistry {
    let mut registry = EssenceRegistry::default();
    fuoco::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_essences_contains_only_fuoco() {
        assert_eq!(default_essences().len(), 1);
    }
}
