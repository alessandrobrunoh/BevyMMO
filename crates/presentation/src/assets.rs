use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

#[derive(AssetCollection, Resource)]
pub struct PlayerAssets {
    #[asset(path = "models/player.glb#Scene0")]
    pub scene: Handle<WorldAsset>,
}

#[derive(AssetCollection, Resource)]
pub struct BossDragonAssets {
    #[asset(path = "models/boss_dragon.glb#Scene0")]
    pub scene: Handle<WorldAsset>,
}

#[derive(AssetCollection, Resource)]
pub struct CreatureAssets {
    #[asset(path = "models/creatures/goblin.glb#Scene0")]
    pub goblin: Handle<WorldAsset>,
    #[asset(path = "models/npcs/merchant.glb#Scene0")]
    pub merchant: Handle<WorldAsset>,
}

#[derive(AssetCollection, Resource)]
pub struct MapAssets {
    #[asset(path = "models/tree_oak.glb#Scene0")]
    pub tree_oak: Handle<WorldAsset>,
}
