//! Plugin per il target indicator (anello rosso sotto il target).

use crate::game_state::Screen;
use bevy::prelude::*;

use super::systems::{cleanup_target_rings, update_target_ring};

pub struct TargetIndicatorPlugin;

impl Plugin for TargetIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_target_ring, cleanup_target_rings)
                .chain()
                .run_if(in_state(Screen::InGame)),
        );
    }
}
