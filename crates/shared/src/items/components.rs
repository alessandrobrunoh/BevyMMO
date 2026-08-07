//! ECS components for inventory and equipment state.
//!
//! These components are replicated by lightyear (see `network::protocol`) so
//! the client renders server-authoritative state. The client never mutates
//! them directly; it sends [`super::events`] commands to request changes.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::registry::ItemId;

/// Dedicated equipment slot.
///
/// Extensible: today only `Weapon` is used, but future variants (helmet, chest,
/// ...) can be added without breaking serialized data, as long as the new
/// variants are appended.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum EquipSlot {
    #[default]
    Weapon,
    // Helmet, Chest, Boots, Ring, ... (reserved for future migrations)
}

/// Number of generic rectangular slots in [`Inventory`].
///
/// Adding slots requires a schema migration because the value is serialized
/// as a fixed-size array on the wire and on disk.
pub const INVENTORY_CAPACITY: usize = 10;

/// Generic inventory of a player.
///
/// Decision: 1 item = 1 slot. No stack count. Each slot either holds an
/// [`ItemId`] reference or is empty. The layout is a fixed-size array so the
/// UI is stable (slot 7 is always slot 7) and serialization is compact.
///
/// The component is replicated and predicted: the client sees server changes
/// as soon as they arrive, and never writes here directly.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Inventory {
    pub slots: [Option<ItemId>; INVENTORY_CAPACITY],
}

/// Current equipment of a player.
///
/// Multi-slot-ready shape even though only `weapon` is populated today
/// (decision #2 of `plans/inventory-system.md`). New fields like `helmet`,
/// `chest`, ... are added by future migrations without changing the existing
/// field semantics.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Equipment {
    pub weapon: Option<ItemId>,
    // helmet, chest, ... default None (added by future migrations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_inventory_has_empty_slots() {
        let inv = Inventory::default();
        assert_eq!(inv.slots.len(), INVENTORY_CAPACITY);
        for slot in &inv.slots {
            assert!(slot.is_none());
        }
    }

    #[test]
    fn default_equipment_has_no_weapon() {
        let eq = Equipment::default();
        assert!(eq.weapon.is_none());
    }

    #[test]
    fn inventory_roundtrips_through_serde() {
        let mut inv = Inventory::default();
        inv.slots[0] = Some(ItemId::new("iron_sword"));
        inv.slots[3] = Some(ItemId::new("potion"));

        let json = serde_json::to_string(&inv).expect("serialize");
        let back: Inventory = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(inv, back);
    }

    #[test]
    fn equip_slot_default_is_weapon() {
        assert_eq!(EquipSlot::default(), EquipSlot::Weapon);
    }
}
