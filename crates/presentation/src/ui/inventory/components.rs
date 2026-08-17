//! Components and markers for the Inventory UI.

use bevy::prelude::*;
use bevymmo_gameplay::items::components::EquipSlot;

/// State for item selection in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySelection {
    Slot(u8),
    Equipment(EquipSlot),
}

/// Marker component for the Inventory Card window root.
#[derive(Component, Debug)]
pub struct InventoryWindow;

/// Button component attached to an inventory grid slot.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemSlotButton {
    pub index: u8,
}

/// Text component inside an `ItemSlotButton` to show item display name.
#[derive(Component, Debug)]
pub struct ItemSlotText {
    pub index: u8,
}

/// Button component attached to one of the equipment slots (Weapon, Helmet, ...).
#[derive(Component, Debug, Clone, Copy)]
pub struct EquipSlotButton {
    pub slot: EquipSlot,
}

/// Text component inside an `EquipSlotButton` to show the equipped item name
/// (or a placeholder dash when empty).
#[derive(Component, Debug)]
pub struct EquipSlotText {
    pub slot: EquipSlot,
}

/// Button component inside the Item Detail Card to equip the selected item.
#[derive(Component, Debug)]
pub struct EquipButton {
    pub slot_index: u8,
}

/// Button component inside the Item Detail Card to unequip the selected slot.
#[derive(Component, Debug)]
pub struct UnequipButton {
    pub slot: EquipSlot,
}

/// Origin of a dragged item: either a generic inventory slot or an equipment
/// slot. Used by the drag-and-drop systems to decide which network command
/// to send on drop (see `super::drag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSlotOrigin {
    Inventory(u8),
    Equipment(EquipSlot),
}

/// Marker for the floating label that follows the cursor while an item is
/// being dragged.
#[derive(Component, Debug)]
pub struct ItemDragGhost;

/// Root of the confirmation dialog shown when an inventory item is dropped
/// outside the inventory window.
#[derive(Component, Debug)]
pub struct DestroyItemDialog;

#[derive(Component, Debug, Clone, Copy)]
pub struct ConfirmDestroyButton {
    pub instance_id: u64,
}

#[derive(Component, Debug)]
pub struct CancelDestroyButton;
