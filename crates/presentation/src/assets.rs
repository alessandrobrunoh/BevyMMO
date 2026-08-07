use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

#[derive(AssetCollection, Resource)]
pub struct PlayerAssets {
    #[asset(path = "models/player.glb#Scene0")]
    pub scene: Handle<WorldAsset>,

    // Assumiamo che Animation0 sia Idle e Animation1 sia Walk
    // (Adegua questi nomi/indici in base al file .glb reale)
    #[asset(path = "models/player.glb#Animation0")]
    pub idle: Handle<AnimationClip>,

    #[asset(path = "models/player.glb#Animation1")]
    pub walk: Handle<AnimationClip>,
}

#[derive(AssetCollection, Resource)]
pub struct BossDragonAssets {
    #[asset(path = "models/boss_dragon.glb#Scene0")]
    pub scene: Handle<WorldAsset>,
}

#[derive(AssetCollection, Resource)]
pub struct MapAssets {
    #[asset(path = "models/tree_oak.glb#Scene0")]
    pub tree_oak: Handle<WorldAsset>,
}
