//! ECS components for inventory and equipment state.
//!
//! These components are replicated by lightyear (see `network::protocol`) so
//! the client renders server-authoritative state. The client never mutates
//! them directly; it sends [`super::events`] commands to request changes.

use serde::{Deserialize, Serialize};

use super::instance::{ItemInstance, ItemInstanceId};

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
/// Adding slots changes the serialized schema because the inventory uses a
/// fixed-size array on the wire and on disk.
pub const INVENTORY_CAPACITY: usize = 30;

/// Generic inventory of a player.
///
/// Decision: 1 item = 1 slot. No stack count. Each slot either holds an
/// [`ItemInstance`] (an esemplare fisico — carries its own id and, for
/// weapons, its own runic inscription) or is empty. The layout is a
/// fixed-size array so the UI is stable (slot 7 is always slot 7) and
/// serialization is compact.
///
/// The component is replicated and predicted: the client sees server changes
/// as soon as they arrive, and never writes here directly.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Inventory {
    pub slots: [Option<ItemInstance>; INVENTORY_CAPACITY],
}

/// Current equipment of a player: one optional item instance per [`EquipSlot`].
///
/// The component is replicated and predicted, same as [`Inventory`]. The
/// client never writes here directly; it sends [`super::events::EquipItemCommand`]
/// / [`super::events::UnequipItemCommand`] and reads the replicated result.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Equipment {
    pub bag: Option<ItemInstance>,
    pub helmet: Option<ItemInstance>,
    pub cape: Option<ItemInstance>,
    pub weapon: Option<ItemInstance>,
    pub armor: Option<ItemInstance>,
    pub offhand: Option<ItemInstance>,
    pub potion: Option<ItemInstance>,
    pub shoes: Option<ItemInstance>,
    pub food: Option<ItemInstance>,
    pub mount: Option<ItemInstance>,
}

impl Equipment {
    /// Reads the item instance currently occupying `slot`, if any.
    pub fn get(&self, slot: EquipSlot) -> &Option<ItemInstance> {
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

    /// Mutable access to the item instance occupying `slot`.
    pub fn get_mut(&mut self, slot: EquipSlot) -> &mut Option<ItemInstance> {
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

    /// Finds which slot (if any) currently holds the physical esemplare
    /// `instance_id` — per-instance, not per-type: se il giocatore ha due
    /// Flame Staff, questo trova quello specifico, non "un" Flame Staff.
    pub fn slot_holding(&self, instance_id: ItemInstanceId) -> Option<EquipSlot> {
        EquipSlot::ALL
            .into_iter()
            .find(|slot| self.get(*slot).as_ref().is_some_and(|item| item.instance_id == instance_id))
    }
}

#[cfg(test)]
mod tests {
    use super::super::registry::ItemId;
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
        inv.slots[0] = Some(ItemInstance::new(ItemId::new("iron_sword")));
        inv.slots[3] = Some(ItemInstance::new(ItemId::new("potion")));

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
            let instance = ItemInstance::new(ItemId::new("test_item"));
            *eq.get_mut(slot) = Some(instance.clone());
            assert_eq!(eq.get(slot), &Some(instance));
        }
    }

    #[test]
    fn slot_holding_finds_the_right_physical_instance() {
        // The ids are set explicitly: `ItemInstance::new` leaves them
        // unassigned now that the database issues them, so two freshly minted
        // copies would be indistinguishable here.
        let mut eq = Equipment::default();
        let mut helmet = ItemInstance::new(ItemId::new("leather_helmet"));
        helmet.instance_id = ItemInstanceId(1);
        let mut other_helmet = ItemInstance::new(ItemId::new("leather_helmet"));
        other_helmet.instance_id = ItemInstanceId(2);

        eq.helmet = Some(helmet.clone());
        assert_eq!(eq.slot_holding(helmet.instance_id), Some(EquipSlot::Helmet));
        // Stesso tipo, esemplare diverso: non deve essere trovato.
        assert_eq!(eq.slot_holding(other_helmet.instance_id), None);
    }

    #[test]
    fn equipment_roundtrips_through_serde() {
        let eq = Equipment {
            weapon: Some(ItemInstance::new(ItemId::new("iron_sword"))),
            mount: Some(ItemInstance::new(ItemId::new("swift_steed"))),
            ..Default::default()
        };

        let json = serde_json::to_string(&eq).expect("serialize");
        let back: Equipment = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(eq, back);
    }
}
