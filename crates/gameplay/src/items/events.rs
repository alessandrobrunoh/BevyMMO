//! Network commands for inventory mutations.
//!
//! The client sends these on `Channel2` (see `network::protocol`); the server
//! validates them against the `ItemRegistry` and the player's `Inventory` /
//! `Equipment` before mutating the authoritative state and persisting.
//!
//! The client never mutates inventory/equipment locally; it only renders the
//! replicated state, exactly like the spell hotbar flow.

use serde::{Deserialize, Serialize};

use super::components::EquipSlot;

/// Client -> server: equip the item currently in `slot_index` (0..10) into
/// the slot declared by its `ItemConfig::equippable_into`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EquipItemCommand {
    pub slot_index: u8,
}

/// Client -> server: remove the item currently in `slot` and return it to the
/// first free inventory slot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UnequipItemCommand {
    pub slot: EquipSlot,
}

/// Client -> server: move the item at slot `from` into slot `to`, swapping if
/// `to` is already occupied.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MoveItemCommand {
    pub from: u8,
    pub to: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equip_command_roundtrips() {
        let cmd = EquipItemCommand { slot_index: 3 };
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: EquipItemCommand = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cmd, back);
    }

    #[test]
    fn unequip_command_roundtrips() {
        let cmd = UnequipItemCommand {
            slot: EquipSlot::Weapon,
        };
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: UnequipItemCommand = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cmd, back);
    }

    #[test]
    fn move_command_roundtrips() {
        let cmd = MoveItemCommand { from: 0, to: 9 };
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: MoveItemCommand = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cmd, back);
    }
}
