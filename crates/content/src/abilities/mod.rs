//! Base-ability content and its registry.

// Existing abilities
pub mod arcane_orb;

pub mod bulwark_strike;
pub mod cleanse;
pub mod ground_break;
pub mod iron_wave;
pub mod meteor_lance;
pub mod mind_burst;
pub mod purge;
pub mod swift_kick;
pub mod warding_bolt;

// Staff family (Q/W/E)
pub mod arcane_bolt;
pub mod arcane_wave;
pub mod great_manifestation;

// Bow family (Q/W/E)
pub mod piercing_barrage;
pub mod power_shot;
pub mod volley;

// Sword family (Q/W/E)
pub mod blade_storm;
pub mod cleave;
pub mod lunge;

// Hammer family (Q/W/E)
pub mod cataclysm;
pub mod crushing_blow;
pub mod ground_slam;

use crate::abilities::BaseAbilityRegistry;

/// Builds the registry containing every base ability shipped by this game build.
pub fn default_base_abilities() -> BaseAbilityRegistry {
    let mut registry = BaseAbilityRegistry::default();

    // Original abilities
    arcane_orb::register(&mut registry);

    bulwark_strike::register(&mut registry);
    ground_break::register(&mut registry);
    iron_wave::register(&mut registry);
    cleanse::register(&mut registry);
    meteor_lance::register(&mut registry);
    mind_burst::register(&mut registry);
    purge::register(&mut registry);
    swift_kick::register(&mut registry);
    warding_bolt::register(&mut registry);

    // Staff family
    arcane_bolt::register(&mut registry);
    arcane_wave::register(&mut registry);
    great_manifestation::register(&mut registry);

    // Bow family
    power_shot::register(&mut registry);
    volley::register(&mut registry);
    piercing_barrage::register(&mut registry);

    // Sword family
    cleave::register(&mut registry);
    lunge::register(&mut registry);
    blade_storm::register(&mut registry);

    // Hammer family
    crushing_blow::register(&mut registry);
    ground_slam::register(&mut registry);
    cataclysm::register(&mut registry);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_abilities_contains_core_abilities() {
        let registry = default_base_abilities();
        assert_eq!(registry.len(), 22); // 10 original + 12 weapon abilities

        // Original abilities
        assert!(registry.contains(&crate::abilities::AbilityId::new("arcane_orb")));

        assert!(registry.contains(&crate::abilities::AbilityId::new("cleanse")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("warding_bolt")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("mind_ward")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("bulwark_strike")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("iron_wave")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("swift_kick")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("ground_break")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("meteor_lance")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("purge")));

        // Staff family
        assert!(registry.contains(&crate::abilities::AbilityId::new("arcane_bolt")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("arcane_wave")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("great_manifestation")));

        // Bow family
        assert!(registry.contains(&crate::abilities::AbilityId::new("power_shot")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("volley")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("piercing_barrage")));

        // Sword family
        assert!(registry.contains(&crate::abilities::AbilityId::new("cleave")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("lunge")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("blade_storm")));

        // Hammer family
        assert!(registry.contains(&crate::abilities::AbilityId::new("crushing_blow")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("ground_slam")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("cataclysm")));
    }
}
