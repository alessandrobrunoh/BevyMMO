//! Screen-space Crowd Control bar (orange, draining) projected above stunned entities.

pub mod components;
mod systems;

use bevy::prelude::*;
use bevymmo_network::network::mode::has_client;

pub struct CrowdControlBarPlugin;

impl Plugin for CrowdControlBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                systems::sync_screen_cc_bars,
                // See `RenderSync`: the projection must see this frame's camera
                // and target transforms, or the bar swims while the player walks.
                systems::update_screen_cc_bars.in_set(crate::renderer::RenderSync::Project),
            )
                .chain()
                .run_if(has_client)
                .run_if(in_gameplay),
        );
        app.add_systems(
            Update,
            systems::cleanup_screen_cc_bars
                .run_if(has_client)
                .run_if(not_in_gameplay),
        );
    }
}

fn in_gameplay(screen: Res<crate::game_state::GameScreen>) -> bool {
    matches!(
        screen.0,
        crate::game_state::Screen::InGame | crate::game_state::Screen::Paused
    )
}

fn not_in_gameplay(screen: Res<crate::game_state::GameScreen>) -> bool {
    !in_gameplay(screen)
}
