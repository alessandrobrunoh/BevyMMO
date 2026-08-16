//! Plugin della death screen.

use bevy::prelude::*;
use bevymmo_network::network::mode::has_client;

use super::systems::{
    handle_respawn_button, setup_death_screen, update_death_screen_visibility,
    update_respawn_button_visuals,
};

/// Marker: root dell'overlay di morte (controlla la visibilità).
#[derive(Component)]
pub struct DeathScreenRoot;

/// Marker: pulsante "Respawn".
#[derive(Component)]
pub struct DeathScreenButton;

pub struct DeathScreenPlugin;

impl Plugin for DeathScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_death_screen);
        app.add_systems(
            Update,
            (
                update_death_screen_visibility,
                handle_respawn_button,
                update_respawn_button_visuals,
            )
                .run_if(has_client),
        );
    }
}
