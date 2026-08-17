//! Root Word content and its registry.

pub mod damage;
pub mod flame;
pub mod frost;
pub mod storm;
pub mod life;
pub mod void;
pub mod stone;

use crate::abilities::RootWordRegistry;

/// Builds the registry containing every root word shipped by this game build.
pub fn default_root_words() -> RootWordRegistry {
    let mut registry = RootWordRegistry::default();
    damage::register(&mut registry);
    flame::register(&mut registry);
    frost::register(&mut registry);
    storm::register(&mut registry);
    life::register(&mut registry);
    void::register(&mut registry);
    stone::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_root_words_contains_all_seven() {
        let reg = default_root_words();
        assert_eq!(reg.len(), 7);
        assert!(reg.contains(&crate::abilities::RootWordId::from("damage")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("flame")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("frost")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("storm")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("life")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("void")));
        assert!(reg.contains(&crate::abilities::RootWordId::from("stone")));
    }

    #[test]
    fn damage_metadata_preserved() {
        let reg = default_root_words();
        let word = reg
            .get(&crate::abilities::RootWordId::from("damage"))
            .unwrap();
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Danno");
        assert_eq!(meta.rune_cost, 1);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn all_root_words_have_stable_ids() {
        let reg = default_root_words();
        let ids = ["damage", "flame", "frost", "storm", "life", "void", "stone"];
        for id in ids.iter() {
            assert!(reg.contains(&crate::abilities::RootWordId::from(*id)), "Missing root word: {id}");
        }
    }

    #[test]
    fn void_has_higher_rune_cost() {
        let reg = default_root_words();
        let word = reg.get(&crate::abilities::RootWordId::from("void")).unwrap();
        assert_eq!(word.metadata().rune_cost, 2);
    }
}
