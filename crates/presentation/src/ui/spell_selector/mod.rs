//! Spell-selection UI — replaces the old free-form Spellbook.
//!
//! The player no longer assigns *any* registered spell to a hotbar key: the
//! legal picks per key are whatever the currently equipped items offer (see
//! `bevymmo_gameplay::items::AvailableSpellChoices`, kept in sync client-side
//! by `crate::spells::available_choices::sync_available_spell_choices`).
//! This window only ever renders — and lets the player click — spells drawn
//! from that pool.

mod components;
mod systems;

use bevy::prelude::*;
use bevymmo_network::network::mode::has_client;

use crate::game_state::{GameScreen, Screen};

#[derive(Resource, Default)]
pub struct SpellSelectorUiState {
    pub is_open: bool,
}

pub struct SpellSelectorUiPlugin;

impl Plugin for SpellSelectorUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpellSelectorUiState>();
        app.add_systems(
            Update,
            (
                systems::toggle_spell_selector,
                systems::update_spell_selector_ui,
                systems::handle_spell_selector_interactions,
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
