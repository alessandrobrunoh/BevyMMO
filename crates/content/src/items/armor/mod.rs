//! Armor content — chestplates, helmets, and boots.

pub mod boots;
pub mod chestplate;
pub mod helmet;
pub mod simple;

use crate::items::ItemRegistry;

/// Registers all armor items with the item registry.
pub fn register(registry: &mut ItemRegistry) {
    chestplate::robust_cuirass::register(registry);
    helmet::warding_helm::register(registry);
    boots::swift_boots::register(registry);
    simple::register(registry);
}
