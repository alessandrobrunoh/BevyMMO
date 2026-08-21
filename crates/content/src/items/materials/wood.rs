//! Gathered oak wood. Stacks in the bag up to [`Inventory::MAX_STACK`].

use bevymmo_props_macro::item;

use crate::items::ItemRegistry;

#[item(
    id = "wood",
    name = "Wood",
    description = "A piece of oak.",
    category = Material,
    rarity = Common,
    tradable = true,
)]
pub struct Wood;

/// Adds this content package to the item registry.
pub fn register(registry: &mut ItemRegistry) {
    Wood::register(registry);
}
