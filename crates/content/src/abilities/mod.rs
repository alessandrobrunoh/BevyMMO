//! Base-ability content and its registry.

pub mod arcane_orb;
pub mod astral_nova;
pub mod cleanse;
pub mod meteor_lance;
pub mod purge;

use crate::abilities::BaseAbilityRegistry;

/// Builds the registry containing every base ability shipped by this game build.
pub fn default_base_abilities() -> BaseAbilityRegistry {
    let mut registry = BaseAbilityRegistry::default();
    arcane_orb::register(&mut registry);
    astral_nova::register(&mut registry);
    cleanse::register(&mut registry);
    meteor_lance::register(&mut registry);
    purge::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_abilities_contains_core_abilities() {
        let registry = default_base_abilities();
        assert_eq!(registry.len(), 5); // arcane_orb + astral_nova + cleanse + meteor_lance + purge
        assert!(registry.contains(&crate::abilities::AbilityId::new("arcane_orb")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("astral_nova")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("cleanse")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("meteor_lance")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("purge")));
    }
}
