//! Scena base: camera, luce direzionale e piano di gioco.
//!
//! Il ciclo di vita è guidato da [`GameScreen`]: la scena viene spawnata una
//! sola volta quando si entra in `InGame`/`Paused` e despawnata (root + figli)
//! quando si torna a `MainMenu`/`Settings`/`Connecting`. Il marker
//! [`GameSceneRoot`] rende lo spawn idempotente.

pub mod systems;

use bevy::prelude::*;

pub struct BaseScenePlugin;

impl Plugin for BaseScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                systems::update_game_scene_lifecycle,
                systems::follow_controlled_player,
            ),
        );
    }
}
