//! Client presentation layer for BevyMMO.

pub mod assets;
pub mod entity;
pub mod game_state;
pub mod map_loader;
pub mod renderer;
pub mod scenes;
pub mod spells;
pub mod ui;
pub mod world;

use assets::{BossDragonAssets, CreatureAssets, MapAssets, PlayerAssets};
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

#[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
pub enum PresentationState {
    #[default]
    Loading,
    Ready,
}

pub struct PresentationCorePlugin;

impl Plugin for PresentationCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<PresentationState>()
            .init_resource::<bevymmo_gameplay::placeables::PlaceableRegistry>()
            .insert_resource(bevymmo_content::status_definitions::default_statuses())
            .add_loading_state(
                LoadingState::new(PresentationState::Loading)
                    .continue_to_state(PresentationState::Ready)
                    .load_collection::<PlayerAssets>()
                    .load_collection::<BossDragonAssets>()
                    .load_collection::<CreatureAssets>()
                    .load_collection::<MapAssets>(),
            )
            .add_systems(Startup, register_presentation_placeables);
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
            crate::world::WorldMapPlugin,
        ));
    }
}

fn register_presentation_placeables(
    mut registry: ResMut<bevymmo_gameplay::placeables::PlaceableRegistry>,
) {
    bevymmo_content::placeable_definitions::register_all(&mut registry);
}

pub mod prelude {
    pub use crate::entity::EntityVisualsPlugin;
    pub use crate::game_state::{
        validate_player_name, ConnectionFailure, ConnectionIntent, ConnectionRequest, GameScreen,
        GameStatePlugin, PlayerNameError, Screen,
    };
    pub use crate::renderer::RendererPlugin;
    pub use crate::scenes::ScenesPlugin;
    pub use crate::spells::SpellsHudPlugin;
    pub use crate::{PresentationCorePlugin, PresentationPlugin};
}
