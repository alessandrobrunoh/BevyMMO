//! Inscription UI for Eidolon Root Words and Ancient Words.
//!
//! Opens on the spellbook key when the equipped weapon or inscribed armor
//! exposes an ability loadout.

mod components;
pub(crate) mod systems;

use bevy::prelude::*;
use bevymmo_network::network::mode::has_client;

use crate::game_state::{not_typing, GameScreen, Screen};

#[derive(Resource, Default)]
pub struct InscriptionUiState {
    pub is_open: bool,
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
                .run_if(has_client)
                .run_if(in_gameplay_or_paused),
        );
    }
}

fn in_gameplay_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}
