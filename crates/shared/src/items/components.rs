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
/// One variant per body/utility slot shown by the inventory UI. Adding a new
/// variant requires: a new [`Equipment`] field, a migration adding the matching
/// DB column, and appending the variant here (existing serialized data stays
/// valid as long as variants are never removed or reordered destructively).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum EquipSlot {
    Bag,
    Helmet,
    Cape,
    #[default]
    Weapon,
    Armor,
    Offhand,
    Potion,
    Shoes,
    Food,
    Mount,
}

impl EquipSlot {
    /// Every equip slot, in the display order used by the inventory UI
    /// (matches the 3x3 grid + Mount layout of the reference design).
    pub const ALL: [EquipSlot; 10] = [
        EquipSlot::Bag,
        EquipSlot::Helmet,
        EquipSlot::Cape,
        EquipSlot::Weapon,
        EquipSlot::Armor,
        EquipSlot::Offhand,
        EquipSlot::Potion,
        EquipSlot::Shoes,
        EquipSlot::Food,
        EquipSlot::Mount,
    ];

    /// Short uppercase label shown above the slot box in the UI.
    pub fn label(&self) -> &'static str {
        match self {
            EquipSlot::Bag => "BAG",
            EquipSlot::Helmet => "HELMET",
            EquipSlot::Cape => "CAPE",
            EquipSlot::Weapon => "WEAPON",
            EquipSlot::Armor => "ARMOR",
            EquipSlot::Offhand => "OFFHAND",
            EquipSlot::Potion => "POTION",
            EquipSlot::Shoes => "SHOES",
            EquipSlot::Food => "FOOD",
            EquipSlot::Mount => "MOUNT",
        }
    }
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

/// Current equipment of a player: one optional item id per [`EquipSlot`].
///
/// The component is replicated and predicted, same as [`Inventory`]. The
/// client never writes here directly; it sends [`super::events::EquipItemCommand`]
/// / [`super::events::UnequipItemCommand`] and reads the replicated result.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Equipment {
    pub bag: Option<ItemId>,
    pub helmet: Option<ItemId>,
    pub cape: Option<ItemId>,
    pub weapon: Option<ItemId>,
    pub armor: Option<ItemId>,
    pub offhand: Option<ItemId>,
    pub potion: Option<ItemId>,
    pub shoes: Option<ItemId>,
    pub food: Option<ItemId>,
    pub mount: Option<ItemId>,
}

impl Equipment {
    /// Reads the item currently occupying `slot`, if any.
    pub fn get(&self, slot: EquipSlot) -> &Option<ItemId> {
        match slot {
            EquipSlot::Bag => &self.bag,
            EquipSlot::Helmet => &self.helmet,
            EquipSlot::Cape => &self.cape,
            EquipSlot::Weapon => &self.weapon,
            EquipSlot::Armor => &self.armor,
            EquipSlot::Offhand => &self.offhand,
            EquipSlot::Potion => &self.potion,
            EquipSlot::Shoes => &self.shoes,
            EquipSlot::Food => &self.food,
            EquipSlot::Mount => &self.mount,
        }
    }

    /// Mutable access to the item occupying `slot`.
    pub fn get_mut(&mut self, slot: EquipSlot) -> &mut Option<ItemId> {
        match slot {
            EquipSlot::Bag => &mut self.bag,
            EquipSlot::Helmet => &mut self.helmet,
            EquipSlot::Cape => &mut self.cape,
            EquipSlot::Weapon => &mut self.weapon,
            EquipSlot::Armor => &mut self.armor,
            EquipSlot::Offhand => &mut self.offhand,
            EquipSlot::Potion => &mut self.potion,
            EquipSlot::Shoes => &mut self.shoes,
            EquipSlot::Food => &mut self.food,
            EquipSlot::Mount => &mut self.mount,
        }
    }

    /// Finds which slot (if any) currently holds `item_id`.
    pub fn slot_holding(&self, item_id: &ItemId) -> Option<EquipSlot> {
        EquipSlot::ALL
            .into_iter()
            .find(|slot| self.get(*slot).as_ref() == Some(item_id))
    }
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

    #[test]
    fn get_and_get_mut_round_trip_for_every_slot() {
        for slot in EquipSlot::ALL {
            let mut eq = Equipment::default();
            assert!(eq.get(slot).is_none());
            *eq.get_mut(slot) = Some(ItemId::new("test_item"));
            assert_eq!(eq.get(slot), &Some(ItemId::new("test_item")));
        }
    }

    #[test]
    fn slot_holding_finds_the_right_slot() {
        let mut eq = Equipment::default();
        eq.helmet = Some(ItemId::new("leather_helmet"));
        assert_eq!(eq.slot_holding(&ItemId::new("leather_helmet")), Some(EquipSlot::Helmet));
        assert_eq!(eq.slot_holding(&ItemId::new("nope")), None);
    }

    #[test]
    fn equipment_roundtrips_through_serde() {
        let mut eq = Equipment::default();
        eq.weapon = Some(ItemId::new("iron_sword"));
        eq.mount = Some(ItemId::new("swift_steed"));

        let json = serde_json::to_string(&eq).expect("serialize");
        let back: Equipment = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(eq, back);
    }
}
