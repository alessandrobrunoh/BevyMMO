//! Inventory UI plugin definition and state.

pub mod components;
pub mod detail;
pub mod drag;
pub mod systems;
pub mod weapon_detail;

use bevy::prelude::*;
use bevymmo_network::network::mode::has_client;

use crate::game_state::{not_typing, GameScreen, Screen};
use components::InventorySelection;
pub use drag::ItemDragState;

/// Global state resource for the Inventory UI.
#[derive(Resource, Default)]
pub struct InventoryUiState {
    pub is_open: bool,
    pub selected: Option<InventorySelection>,
}

pub struct InventoryUiPlugin;

impl Plugin for InventoryUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryUiState>();
        app.init_resource::<ItemDragState>();
        app.add_systems(
            Update,
            (
                // Only the *toggle* is gated by typing focus — the window,
                // once open, must keep rendering/dragging normally even if
                // the player also opens chat, so the gate does not apply to
                // the rest of this chain.
                systems::toggle_inventory.run_if(not_typing),
                systems::update_inventory_ui,
                systems::handle_inventory_interactions,
                drag::start_item_drag,
                drag::update_item_drag,
                drag::end_item_drag,
                drag::handle_destroy_dialog,
            )
                .chain()
                .run_if(has_client)
                .run_if(in_gameplay_or_paused),
        );
    }
}

fn in_gameplay_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}
