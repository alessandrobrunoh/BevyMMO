//! Concrete built-in item implementations.
//!
//! Each submodule is a self-contained `Item` trait implementation with no
//! transport/rendering dependencies, so the registry in the binary (or any
//! other crate) can compose them freely. Mirrors `crate::spells_impl`.

pub mod field_rations;
pub mod magic_staff;
pub mod iron_plate_armor;
pub mod iron_sword;
pub mod leather_helmet;
pub mod quick_flask;
pub mod storm_hammer;
pub mod swift_boots;
pub mod swift_steed;
pub mod travelers_bag;
pub mod travelers_cape;
pub mod wooden_shield;

use std::sync::Arc;

use crate::items::registry::ItemRegistry;

/// Registers every item definition available to the current game build.
///
/// Called once at startup by both the server (authoritative source) and the
/// client (UI rendering). Keeping the list in `shared` guarantees both sides
/// agree on what items exist. One reference item per [`crate::items::components::EquipSlot`]
/// so every slot in the inventory UI has something equippable to test with.
/// Builds the registry containing every entry this build ships.
///
/// Returns the registry rather than filling a Bevy `Resource`: the
/// SpacetimeDB module has no `Startup` schedule and no ECS to put one in.
/// `bevymmo_shared` wraps this in a system for the client.
pub fn default_items() -> ItemRegistry {
    #[allow(unused_mut)]
    let mut registry = ItemRegistry::default();
    registry.register(Arc::new(iron_sword::IronSword::new()));
    registry.register(Arc::new(magic_staff::MagicStaff));
    registry.register(Arc::new(storm_hammer::StormHammer));
    registry.register(Arc::new(leather_helmet::LeatherHelmet::new()));
    registry.register(Arc::new(travelers_cape::TravelersCape::new()));
    registry.register(Arc::new(iron_plate_armor::IronPlateArmor::new()));
    registry.register(Arc::new(wooden_shield::WoodenShield::new()));
    registry.register(Arc::new(quick_flask::QuickFlask::new()));
    registry.register(Arc::new(swift_boots::SwiftBoots::new()));
    registry.register(Arc::new(field_rations::FieldRations::new()));
    registry.register(Arc::new(swift_steed::SwiftSteed::new()));
    registry.register(Arc::new(travelers_bag::TravelersBag::new()));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::registry::ItemId;

    #[test]
    fn default_items_adds_iron_sword() {
        let registry = default_items();
        assert!(
            registry.contains(&ItemId::new(iron_sword::IronSword::ID)),
            "iron_sword must be registered by default_items"
        );
        assert!(!registry.is_empty());
    }
}
