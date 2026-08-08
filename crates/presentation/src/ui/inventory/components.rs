//! Components and markers for the Inventory UI.

use bevy::prelude::*;
use bevymmo_shared::items::components::EquipSlot;

/// State for item selection in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySelection {
    Slot(u8),
    Weapon,
}

/// Marker component for the Inventory Card window root.
#[derive(Component, Debug)]
pub struct InventoryWindow;

/// Button component attached to one of the 10 inventory grid slots (0..10).
#[derive(Component, Debug)]
pub struct ItemSlotButton {
    pub index: u8,
}

/// Text component inside an `ItemSlotButton` to show item display name.
#[derive(Component, Debug)]
pub struct ItemSlotText {
    pub index: u8,
}

/// Button component attached to the special weapon equipment slot.
#[derive(Component, Debug)]
pub struct WeaponSlotButton;

/// Text component inside `WeaponSlotButton` to show equipped weapon display name.
#[derive(Component, Debug)]
pub struct WeaponSlotText;

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
