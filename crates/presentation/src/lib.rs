//! Client presentation layer for BevyMMO.

pub mod assets;
pub mod entity;
pub mod game_state;
mod harvest;
pub mod map_loader;
pub mod renderer;
pub mod scenes;
pub mod spells;
pub mod ui;
pub mod world;

use assets::{BossDragonAssets, CreatureAssets, MapAssets, PlayerAssets, WeaponAssets};
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
            .insert_resource(bevymmo_content::spell_definitions::default_spells())
            .insert_resource(bevymmo_content::item_definitions::default_items())
            .insert_resource(bevymmo_content::ability_definitions::default_base_abilities())
            .insert_resource(bevymmo_content::ancient_word_definitions::default_ancient_words())
            .insert_resource(bevymmo_content::root_word_definitions::default_root_words())
            .add_loading_state(
                LoadingState::new(PresentationState::Loading)
                    .continue_to_state(PresentationState::Ready)
                    .load_collection::<PlayerAssets>()
                    .load_collection::<BossDragonAssets>()
                    .load_collection::<CreatureAssets>()
                    .load_collection::<MapAssets>()
                    .load_collection::<WeaponAssets>(),
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
    pub use crate::game_state::{
        ConnectionFailure, ConnectionIntent, ConnectionRequest, GameStatePlugin, PauseOverlay,
        PlayerNameError, Screen, validate_player_name,
    };
    pub use crate::renderer::RendererPlugin;
    pub use crate::scenes::ScenesPlugin;
    pub use crate::spells::SpellsHudPlugin;
    pub use crate::{PresentationCorePlugin, PresentationPlugin};
}
