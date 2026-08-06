//! Scoreboard mostrato mentre il tasto configurato è premuto.

use bevy::prelude::*;

use super::systems;
use crate::ui::text::spawn_text;

/// Marker: root della scoreboard.
#[derive(Component)]
pub struct ScoreboardUi;

/// Marker: pannello contenente la lista nomi (i suoi discendenti vengono
/// rigenerati solo quando serve).
#[derive(Component)]
pub struct ScoreboardPanel;

/// Stato della scoreboard per detectare cambi e rebuildare la lista solo
/// quando necessario (apertura pannello o cambio del set di nomi).
#[derive(Resource, Default)]
pub struct ScoreboardState {
    pub open: bool,
    pub names: Vec<String>,
}

pub struct ScoreboardPlugin;

impl Plugin for ScoreboardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScoreboardState>();
        app.add_systems(Startup, systems::setup_scoreboard);
        app.add_systems(
            Update,
            systems::update_scoreboard.run_if(crate::ui::systems::in_gameplay),
        );
    }
}
