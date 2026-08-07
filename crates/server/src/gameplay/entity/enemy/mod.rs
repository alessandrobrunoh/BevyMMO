//! Server-authoritative enemy systems and plugin.

use bevy::prelude::*;

use bevymmo_shared::entity::enemy::components::Respawning;
use bevymmo_shared::network::mode::has_server;

use crate::gameplay::entity::enemy::systems::{
    enemy_auto_cast_attack, enemy_chase, enemy_respawn, schedule_enemy_respawn,
};

pub mod systems;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Respawning>();
        app.add_systems(
            FixedUpdate,
            (
                enemy_chase,
                enemy_auto_cast_attack,
                schedule_enemy_respawn,
                enemy_respawn,
            )
                .chain()
                .run_if(has_server),
        );
    }
}
