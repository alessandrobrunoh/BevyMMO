//! Generic id -> value catalog, shared by every content registry in this
//! crate (spells, items, base abilities, essences, root/ancient words,
//! modifiers, statuses, weapon families).
//!
//! Before this module existed, each of those nine types hand-wrote the same
//! `HashMap<Id, V>` wrapper with `register`/`get`/`contains`/`len`/
//! `is_empty` (and, for two of them, a `sorted_*` method). [`Registry`] is
//! that shared shape as a single generic type; each catalog still wraps it
//! in its own newtype and keeps its own public method names and return
//! types (some clone an `Arc<dyn Trait>` out on `get`, one borrows a plain
//! metadata struct instead) so this consolidation is purely mechanical —
//! no caller-visible behavior changed.

use std::collections::HashMap;
use std::hash::Hash;

/// A `HashMap<K, V>` with the catalog-style surface every content registry
/// in this crate needs: insert-by-key, lookup, membership, size, and a
/// stable sort for UI display order.
#[derive(Debug, Clone)]
pub struct Registry<K, V> {
    entries: HashMap<K, V>,
}

impl<K, V> Default for Registry<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash, V> Registry<K, V> {
    /// Inserts `value` under `key`. Overwrites any existing entry, matching
    /// every wrapper's current "last registration wins" behavior.
    pub fn insert(&mut self, key: K, value: V) {
        self.entries.insert(key, value);
    }

    /// Looks up an entry by key. Borrowed, not cloned: callers that need an
    /// owned value (most of the `Arc<dyn Trait>` catalogs) clone it
    /// themselves in their wrapper's `get`, matching their existing public
    /// return type.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All entries as owned `(key, value)` pairs, sorted with `cmp`.
    ///
    /// Takes a full comparator rather than a key-extraction closure so a
    /// catalog can sort by a borrowed field (e.g. `display_name() -> &str`)
    /// without that borrow having to outlive the closure — `ItemRegistry`
    /// and `SpellRegistry` both sort by display name today, and this keeps
    /// that exact ordering.
    pub fn sorted_by<F>(&self, mut cmp: F) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
        F: FnMut(&V, &V) -> std::cmp::Ordering,
    {
        let mut list: Vec<(K, V)> = self
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        list.sort_by(|a, b| cmp(&a.1, &b.1));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_get_returns_the_value() {
        let mut registry: Registry<u32, &str> = Registry::default();
        registry.insert(1, "one");
        assert_eq!(registry.get(&1), Some(&"one"));
        assert_eq!(registry.get(&2), None);
    }

    #[test]
    fn insert_overwrites_an_existing_key() {
        let mut registry: Registry<u32, &str> = Registry::default();
        registry.insert(1, "one");
        registry.insert(1, "uno");
        assert_eq!(registry.get(&1), Some(&"uno"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn contains_len_and_is_empty_track_the_entries() {
        let mut registry: Registry<u32, &str> = Registry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(!registry.contains(&1));

        registry.insert(1, "one");
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains(&1));
    }

    #[test]
    fn sorted_by_orders_entries_using_the_comparator() {
        let mut registry: Registry<u32, &str> = Registry::default();
        registry.insert(1, "banana");
        registry.insert(2, "apple");
        registry.insert(3, "cherry");

        let sorted = registry.sorted_by(|a, b| a.cmp(b));

        assert_eq!(
            sorted,
            vec![(2, "apple"), (1, "banana"), (3, "cherry")]
        );
    }
}
