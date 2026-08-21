//! Static crafting recipe declared on an output item.

use super::registry::ItemId;

/// One ingredient in a [`CraftRecipe`], amounts are per crafted item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftIngredient {
    pub item_id: ItemId,
    pub amount: u32,
}

/// How to make one copy of an item. `None` on [`super::Item::craft_recipe`] means
/// the item is unique and not craftable.
#[derive(Debug, Clone, PartialEq)]
pub struct CraftRecipe {
    pub ingredients: Vec<CraftIngredient>,
    pub channel_seconds: f32,
}
