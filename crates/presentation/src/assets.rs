use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

#[derive(AssetCollection, Resource)]
pub struct PlayerAssets {

    // Assumiamo che Animation0 sia Idle e Animation1 sia Walk
    // (Adegua questi nomi/indici in base al file .glb reale)
    #[asset(path = "models/player.glb#Animation0")]
    pub idle: Handle<AnimationClip>,

    #[asset(path = "models/player.glb#Animation1")]
    pub walk: Handle<AnimationClip>,
}
