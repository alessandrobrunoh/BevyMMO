//! Root Word content and its registry.

pub mod damage;

use crate::abilities::RootWordRegistry;

/// Builds the registry containing every root word shipped by this game build.
pub fn default_root_words() -> RootWordRegistry {
    let mut registry = RootWordRegistry::default();
    damage::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_root_words_contains_damage() {
        let reg = default_root_words();
        assert!(reg.contains(&crate::abilities::RootWordId::from("damage")));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn root_word_metadata_is_correct() {
        let reg = default_root_words();
        let word = reg
            .get(&crate::abilities::RootWordId::from("damage"))
            .unwrap();
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Danno");
        assert_eq!(meta.rune_cost, 1);
        assert!(!meta.description.is_empty());
    }
}
