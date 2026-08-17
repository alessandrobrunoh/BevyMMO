//! Base-ability content and its registry.

pub mod arcane_orb;
pub mod astral_nova;
pub mod bulwark_strike;
pub mod ground_break;
pub mod iron_wave;
pub mod cleanse;
pub mod meteor_lance;
pub mod mind_burst;
pub mod purge;
pub mod swift_kick;
pub mod warding_bolt;

use crate::abilities::BaseAbilityRegistry;

/// Builds the registry containing every base ability shipped by this game build.
pub fn default_base_abilities() -> BaseAbilityRegistry {
    let mut registry = BaseAbilityRegistry::default();
    arcane_orb::register(&mut registry);
    astral_nova::register(&mut registry);
    bulwark_strike::register(&mut registry);
    ground_break::register(&mut registry);
    iron_wave::register(&mut registry);
    cleanse::register(&mut registry);
    meteor_lance::register(&mut registry);
    mind_burst::register(&mut registry);
    purge::register(&mut registry);
    swift_kick::register(&mut registry);
    warding_bolt::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_abilities_contains_core_abilities() {
        let registry = default_base_abilities();
        assert_eq!(registry.len(), 11); // weapon, armor and utility abilities
        assert!(registry.contains(&crate::abilities::AbilityId::new("arcane_orb")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("astral_nova")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("cleanse")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("warding_bolt")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("mind_ward")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("bulwark_strike")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("iron_wave")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("swift_kick")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("ground_break")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("meteor_lance")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("purge")));
    }
}
