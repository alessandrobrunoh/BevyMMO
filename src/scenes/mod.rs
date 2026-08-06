//! Scene plugin — collects available scenes in the game.

pub mod base;

use bevy::prelude::*;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(base::BaseScenePlugin);
    }
}
