//! Inscription UI — the Eidolon counterpart of `crate::ui::spell_selector`.
//!
//! Shown instead of the spell selector when the equipped weapon has Eidolon
//! gestures (`Item::ability_loadout()`); the two share the same toggle key,
//! each independently checking the equipped weapon before opening (mirrors
//! `crate::spells::input`/`crate::spells::eidolon_input` splitting Q/W/E the
//! same way).

mod components;
mod systems;

use bevy::prelude::*;
use bevymmo_network::network::mode::has_client;

use crate::game_state::{GameScreen, Screen};

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
                systems::toggle_inscription_window,
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
