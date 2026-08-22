//! Crafting rules: recipe cost, channel time, bag preview and apply.
//!
//! Pure functions. The SpacetimeDB module applies them inside a tick; the
//! client UI uses the same preview so the confirm dialog cannot disagree
//! with the reducer.

pub mod components;
pub mod formulas;

pub use crate::items::recipe::{CraftIngredient, CraftRecipe};
pub use components::ActiveCraft;
pub use formulas::{
    apply_craft, channel_duration, max_craftable, preview_craft, scaled_cost, CraftError, CraftPlan,
};
