//! Item content and its registry.

pub mod magic_staff;

use crate::items::registry::ItemRegistry;

/// Builds the registry containing every item shipped by this game build.
pub fn default_items() -> ItemRegistry {
    let mut registry = ItemRegistry::default();
    magic_staff::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::registry::ItemId;

    #[test]
    fn default_items_contains_only_magic_staff() {
        let registry = default_items();
        assert!(registry.contains(&ItemId::new(magic_staff::MagicStaff::ID)));
        assert_eq!(registry.len(), 1);
    }
}
