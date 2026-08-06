mod components;
mod systems;

use bevy::prelude::*;
use bevymmo_shared::network::mode::has_client;
use bevymmo_shared::spells::SpellId;

use crate::game_state::{GameScreen, Screen};

#[derive(Resource, Default)]
pub struct SpellbookUiState {
    pub is_open: bool,
    pub selected_spell: Option<SpellId>,
}

pub struct SpellbookUiPlugin;

impl Plugin for SpellbookUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpellbookUiState>();
        app.add_systems(
            Update,
            (
                systems::toggle_spellbook,
                systems::update_spellbook_ui,
                systems::handle_spellbook_interactions,
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
