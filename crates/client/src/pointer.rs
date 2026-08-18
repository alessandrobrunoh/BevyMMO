//! Shared HUD-vs-world pointer policy.
//!
//! Inventory, chat, the hotbar and inscription all sit on top of the world.
//! World click systems (move, target, NPC pick) must not fire when the pointer
//! is already claimed by one of those nodes.

use bevy::prelude::Resource;

/// `true` when a gameplay HUD node is currently Pressed or Hovered.
///
/// Hovered is required: a right-click on a hovered inventory slot must not
/// write a move target, even though that click never Pressed a HUD button.
pub fn world_pointer_blocked(ui_pressed: bool, ui_hovered: bool) -> bool {
    ui_pressed || ui_hovered
}

/// Set once per frame from UI `Interaction`s. World-click systems early-out
/// when [`hud_wants_pointer`] is true.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerOnHud(pub bool);

/// Shared early-out used by move, targeting and NPC pick.
pub fn hud_wants_pointer(pointer: &PointerOnHud) -> bool {
    pointer.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_pointer_does_not_block_the_world() {
        assert!(!world_pointer_blocked(false, false));
        assert!(!hud_wants_pointer(&PointerOnHud(false)));
    }

    #[test]
    fn pressed_hud_blocks_the_world() {
        assert!(world_pointer_blocked(true, false));
        assert!(hud_wants_pointer(&PointerOnHud(true)));
    }

    #[test]
    fn hovered_hud_blocks_the_world() {
        assert!(world_pointer_blocked(false, true));
    }

    #[test]
    fn pressed_and_hovered_still_blocks() {
        assert!(world_pointer_blocked(true, true));
    }
}
