mod components;
mod systems;

use crate::game_state::{GameScreen, Screen};
use crate::network::mode::has_client;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct SpellbookUiState {
    pub is_open: bool,
    pub selected_spell: Option<crate::plugins::spells::SpellId>,
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
                systems::handle_spell_selection,
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
