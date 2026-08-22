//! Gathered copper. Stacks in the bag up to [`Inventory::MAX_STACK`].

use bevymmo_props_macro::item;

use crate::items::ItemRegistry;

#[item(
    id = "copper",
    name = "Copper",
    description = "A lump of copper ore.",
    category = Material,
    rarity = Common,
    tradable = true,
)]
pub struct Copper;

/// Adds this content package to the item registry.
pub fn register(registry: &mut ItemRegistry) {
    Copper::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::Item;

    #[test]
    fn icon_is_unassigned_until_selected() {
        assert!(Copper.icon().is_none());
    }
}
