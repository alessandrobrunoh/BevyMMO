//! Plugin della death screen.

use bevy::prelude::*;

use crate::game_state::{in_gameplay, Screen};

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
                update_death_screen_visibility
                    .run_if(in_gameplay.or_eager(state_changed::<Screen>)),
                handle_respawn_button,
                update_respawn_button_visuals,
            ),
        );
    }
}
