//! Enemy: entità controllata dal server (AI).

pub mod components;
pub mod debug;
pub mod spawn;
pub mod systems;

use bevy::prelude::*;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (systems::enemy_chase, systems::enemy_auto_cast_attack)
                .chain()
                .run_if(crate::network::mode::has_server),
        );
        debug::client_debug_systems(app);
    }
}
