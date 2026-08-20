//! Inscription UI for Eidolon Root Words and Ancient Words.
//!
//! Opens on the spellbook key when the equipped weapon or inscribed armor
//! exposes an ability loadout.

mod components;
pub(crate) mod systems;

use bevy::prelude::*;

use crate::game_state::{in_gameplay, not_typing};

#[derive(Resource, Default)]
pub struct InscriptionUiState {
    pub is_open: bool,
    /// Vertical offset of the window's scroll view. Restored when the panel
    /// is rebuilt after an equipment replica, so clicking a toggle does not
    /// yank the user back to the top.
    scroll: f32,
    /// Last equipment drawn into the open window. Identical replicas skip
    /// a rebuild (the stdb mirror re-inserts equipment on unrelated row events).
    shown_equipment: Option<bevymmo_gameplay::items::components::Equipment>,
}

pub struct InscriptionUiPlugin;

impl Plugin for InscriptionUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InscriptionUiState>();
        app.add_systems(
            Update,
            (
                // Only the toggle is typing-gated — see the identical note
                // in `ui::inventory::mod`.
                systems::toggle_inscription_window.run_if(not_typing),
                systems::refresh_inscription_window_on_equipment_change,
                systems::handle_inscription_interactions,
            )
                .chain()
                .run_if(in_gameplay),
        );
    }
}
