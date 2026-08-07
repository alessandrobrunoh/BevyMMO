//! Client presentation layer for BevyMMO.

pub mod assets;
pub mod entity;
pub mod game_state;
pub mod player_movement;
pub mod renderer;
pub mod scenes;
pub mod spells;
pub mod ui;
pub mod world;

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use assets::PlayerAssets;

#[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
pub enum PresentationState {
    #[default]
    Loading,
    Ready,
}

pub struct PresentationCorePlugin;

impl Plugin for PresentationCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<PresentationState>().add_loading_state(
            LoadingState::new(PresentationState::Loading)
                .continue_to_state(PresentationState::Ready)
                .load_collection::<PlayerAssets>(),
        );
    }
}

pub struct PresentationPlugin;

impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            crate::ui::UiPlugin,
            crate::scenes::ScenesPlugin,
            crate::renderer::RendererPlugin,
            crate::entity::EntityVisualsPlugin,
            crate::spells::SpellsHudPlugin,
            crate::player_movement::PlayerMovementPredictionPlugin,
            crate::world::WorldMapPlugin,
        ));
    }
}

pub mod prelude {
    pub use crate::entity::EntityVisualsPlugin;
    pub use crate::player_movement::PlayerMovementPredictionPlugin;
    pub use crate::spells::SpellsHudPlugin;
    pub use crate::game_state::{
        validate_player_name, ConnectionFailure, ConnectionIntent, ConnectionRequest, GameScreen,
        GameStatePlugin, PlayerNameError, Screen,
    };
    pub use crate::renderer::RendererPlugin;
    pub use crate::scenes::ScenesPlugin;
    pub use crate::{PresentationCorePlugin, PresentationPlugin};
}
