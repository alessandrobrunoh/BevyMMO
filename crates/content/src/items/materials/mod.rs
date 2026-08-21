//! Gathered crafting materials.

pub mod copper;
pub mod wood;

use crate::items::ItemRegistry;

pub fn register(registry: &mut ItemRegistry) {
    wood::register(registry);
    copper::register(registry);
}
